//! High-level Rust bindings for the Tailscale library.
//!
//! This module provides safe, idiomatic Rust wrappers around the underlying
//! Tailscale C API, enabling easy integration of Tailscale networking into
//! Rust applications.

use std::{
    ffi::{CStr, CString, FromBytesUntilNulError, NulError},
    io::{Read, Write},
    net::{AddrParseError, IpAddr, Ipv4Addr, Ipv6Addr},
    os::fd::{AsFd, AsRawFd, FromRawFd, OwnedFd},
    path::PathBuf,
    str::{FromStr, Utf8Error},
    sync::Arc,
    task::Poll,
};

use crate::sys::{TailscaleListener, modern::*};

use thiserror::Error;
use tokio::{
    io::{AsyncRead, AsyncWrite, unix::AsyncFd},
    task::JoinError,
};
use tracing::{debug, error};

/// Network protocol type for Tailscale connections.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkType {
    /// TCP protocol
    Tcp,
    /// UDP protocol
    Udp,
}

impl NetworkType {
    /// Returns the string representation of the network type.
    fn as_str(&self) -> &'static str {
        match self {
            NetworkType::Tcp => "tcp",
            NetworkType::Udp => "udp",
        }
    }
}

impl std::fmt::Display for NetworkType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Errors that can occur when working with Tailscale.
#[derive(Debug, Error)]
pub enum TailscaleError {
    #[error("failed to create Tailscale instance")]
    CreateTailscale,

    #[error("spawning background thread failed")]
    SpawnBlockingFailed(#[from] JoinError),

    #[error("could not parse address: {0}")]
    AddrParseError(String, AddrParseError),

    #[error("invalid utf-8 string")]
    Utf8Error(#[from] NulError),

    #[error("missing null terminator")]
    NullError(#[from] FromBytesUntilNulError),

    #[error("invalid utf-8 string")]
    Utf8ContentError(#[from] Utf8Error),

    #[error("invalid listen address given")]
    InvalidAddress(#[from] std::io::Error),

    #[error("invalid ip addresses returned: {0}")]
    InvalidIpAdresses(String),

    #[error("failed to recvmsg")]
    Recvmsg,

    #[error("with control message")]
    ControlMessage,

    #[error("Failed to set hostname")]
    SetHostname,

    #[error("Failed to set dir")]
    SetDir,

    #[error("Failed to set auth key")]
    SetAuthKey,

    #[error("Failed to set ephemeral status")]
    SetEphemeral,

    #[error("Failed to set log destination")]
    SetLogFd,

    #[error("failed to bring up Tailscale connection: {0}")]
    UpFailed(String),

    #[error("failed to create listener on {network}://{addr}: {message}")]
    ListenFailed {
        network: String,
        addr: String,
        message: String,
    },

    #[error("failed to dial {network}://{addr}: {message}")]
    DialFailed {
        network: String,
        addr: String,
        message: String,
    },

    #[error("failed to accept connection: {0}")]
    AcceptFailed(String),

    #[error("tailscale error: {0}")]
    Tailscale(String),
}

/// A specialized `Result` type for Tailscale operations.
pub type Result<T> = std::result::Result<T, TailscaleError>;

/// Configuration for Tailscale logging output.
#[derive(Default)]
pub enum LogConfig {
    /// Use Tailscale's default logging behavior.
    #[default]
    Default,
    /// Write logs to a custom log destination.
    /// The log destination will be owned and kept alive for the Tailscale instance lifetime.
    Fd(OwnedFd),
    /// Discard all log output.
    Discard,
}

/// Builder for configuring and creating a Tailscale instance.
///
/// Use this builder to set various configuration options before
/// creating a Tailscale connection.
///
/// # Examples
///
/// ```no_run
/// # use tailscale2::Tailscale;
/// let ts = Tailscale::builder()
///     .hostname("my-host")
///     .ephemeral(true)
///     .build()?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Default)]
pub struct TailscaleBuilder {
    ephemeral: bool,
    hostname: Option<String>,
    dir: Option<PathBuf>,
    auth_key: Option<String>,
    log_config: LogConfig,
}

impl TailscaleBuilder {
    /// Builds and returns a configured Tailscale instance.
    ///
    /// # Errors
    ///
    /// Returns an error if any of the configuration options fail to be set.
    pub fn build(&mut self) -> Result<Arc<Tailscale>> {
        debug!("creating new Tailscale instance");
        let sd = unsafe { tailscale_new() };
        if sd == 0 {
            return Err(TailscaleError::CreateTailscale);
        }
        debug!(sd, "Tailscale instance created");

        if self.ephemeral {
            debug!("setting ephemeral mode");
            let ret = unsafe { tailscale_set_ephemeral(sd, self.ephemeral as _) };
            if ret != 0 {
                return Err(TailscaleError::SetEphemeral);
            }
        }

        if let Some(path) = &self.dir {
            debug!(path = %path.display(), "setting state directory");
            let path_s = path.display().to_string();
            let path_cs = CString::new(path_s)?;
            let ret = unsafe { tailscale_set_dir(sd, path_cs.as_ptr() as *mut _) };
            if ret != 0 {
                return Err(TailscaleError::SetDir);
            }
        };

        if let Some(hostname) = &self.hostname {
            debug!(%hostname, "setting hostname");
            let c_hostname = CString::new(hostname.clone())?;
            let ret = unsafe { tailscale_set_hostname(sd, c_hostname.as_ptr()) };
            if ret != 0 {
                return Err(TailscaleError::SetHostname);
            }
        }
        if let Some(auth_key) = &self.auth_key {
            debug!("setting auth key");
            let c_auth_key = CString::new(auth_key.clone())?;
            let ret = unsafe { tailscale_set_authkey(sd, c_auth_key.as_ptr()) };
            if ret != 0 {
                return Err(TailscaleError::SetAuthKey);
            }
        }

        // Handle log configuration
        let log_fd = match std::mem::take(&mut self.log_config) {
            LogConfig::Default => {
                debug!("using default Tailscale logging");
                // Don't call tailscale_set_logfd, use default logging
                None
            }
            LogConfig::Fd(owned_fd) => {
                let fd = owned_fd.as_raw_fd();
                debug!(fd, "setting custom log destination");
                let ret = unsafe { tailscale_set_logfd(sd, fd) };
                if ret != 0 {
                    return Err(TailscaleError::SetLogFd);
                }
                Some(owned_fd)
            }
            LogConfig::Discard => {
                debug!("disabling Tailscale logging");
                let ret = unsafe { tailscale_set_logfd(sd, -1) };
                if ret != 0 {
                    return Err(TailscaleError::SetLogFd);
                }
                None
            }
        };

        debug!("Tailscale instance built successfully");
        Ok(Arc::new(Tailscale {
            sd,
            _log_fd: log_fd,
        }))
    }

    /// Sets the authentication key for this Tailscale instance.
    ///
    /// # Arguments
    ///
    /// * `key` - The Tailscale authentication key
    pub fn auth_key(&mut self, key: impl Into<String>) -> &mut Self {
        self.auth_key = Some(key.into());
        self
    }

    /// Sets whether this node should be ephemeral.
    ///
    /// Ephemeral nodes are automatically removed from the network when they go offline.
    ///
    /// # Arguments
    ///
    /// * `ephemeral` - Whether the node should be ephemeral
    pub fn ephemeral(&mut self, ephemeral: bool) -> &mut Self {
        self.ephemeral = ephemeral;
        self
    }

    /// Sets the hostname for this Tailscale node.
    ///
    /// # Arguments
    ///
    /// * `hostname` - The desired hostname for the node
    pub fn hostname(&mut self, hostname: impl Into<String>) -> &mut Self {
        self.hostname = Some(hostname.into());
        self
    }

    /// Sets the state directory for Tailscale to store its configuration.
    ///
    /// # Arguments
    ///
    /// * `dir` - Path to the directory where Tailscale should store its state
    pub fn dir(&mut self, dir: impl Into<PathBuf>) -> &mut Self {
        self.dir = Some(dir.into());
        self
    }

    /// Sets a custom log destination for Tailscale logging output.
    ///
    /// # Arguments
    ///
    /// * `destination` - A log destination that implements `AsRawFd` (e.g., `File`, `OwnedFd`)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use std::fs::File;
    /// # use tailscale2::Tailscale;
    /// let log_file = File::create("/tmp/tailscale.log")?;
    /// let ts = Tailscale::builder()
    ///     .log_destination(log_file)
    ///     .build()?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn log_destination(&mut self, destination: impl Into<OwnedFd>) -> &mut Self {
        self.log_config = LogConfig::Fd(destination.into());
        self
    }

    /// Disables all Tailscale logging output.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tailscale2::Tailscale;
    /// let ts = Tailscale::builder()
    ///     .log_discard()
    ///     .build()?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn log_discard(&mut self) -> &mut Self {
        self.log_config = LogConfig::Discard;
        self
    }
}

/// A Tailscale network listener.
///
/// This listener can accept incoming connections from other nodes on the Tailscale network.
pub struct Listener {
    ln: TailscaleListener,
    _tailscale: Arc<Tailscale>,
}

pub type TailscaleConn = libc::c_int;

/// A connection accepted from a Tailscale listener.
///
/// Implements `AsyncRead` and `AsyncWrite` for async I/O.
pub struct Connection {
    listener: Option<Arc<Listener>>,
    conn: AsyncFd<OwnedFd>,
}

impl Connection {
    /// Returns the remote IP address of this connection.
    ///
    /// # Errors
    ///
    /// Returns an error if the remote address cannot be retrieved or parsed.
    pub fn remote_addr(&self) -> Result<Option<IpAddr>> {
        let Some(listener) = &self.listener else {
            return Ok(None);
        };

        let conn_fd = self.conn.as_raw_fd();
        let buf = [0u8; 128];
        let ret = unsafe {
            tailscale_getremoteaddr(listener.ln, conn_fd, buf.as_ptr() as *mut _, buf.len())
        };

        if ret != 0 {
            let error_message = listener._tailscale.get_error_message()?;
            return Err(TailscaleError::Tailscale(error_message));
        }

        let s = CStr::from_bytes_until_nul(&buf[..])?;
        let s = s.to_str()?;

        let addr =
            IpAddr::from_str(s).map_err(|e| TailscaleError::AddrParseError(s.to_string(), e))?;

        Ok(Some(addr))
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        debug!("dropping connection");
        // AsyncFd<OwnedFd> automatically closes the fd on drop
    }
}

impl Read for Connection {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let fd = self.conn.get_ref().as_fd();
        nix::unistd::read(fd, buf).map_err(|errno| std::io::Error::from_raw_os_error(errno as i32))
    }
}

impl Write for Connection {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let fd = self.conn.get_ref().as_fd();
        nix::unistd::write(fd, buf).map_err(|errno| std::io::Error::from_raw_os_error(errno as i32))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl AsyncRead for Connection {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        loop {
            let mut guard = match self.conn.poll_read_ready(cx) {
                Poll::Ready(Ok(guard)) => guard,
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => return Poll::Pending,
            };

            let fd = self.conn.get_ref().as_fd();

            // Safety: We're reading into the unfilled portion of the buffer
            // and will call assume_init and advance after a successful read
            let unfilled = unsafe {
                let slice = buf.unfilled_mut();
                std::slice::from_raw_parts_mut(slice.as_mut_ptr() as *mut u8, slice.len())
            };

            match nix::unistd::read(fd, unfilled) {
                Ok(n) => {
                    unsafe {
                        buf.assume_init(n);
                    }
                    buf.advance(n);
                    return Poll::Ready(Ok(()));
                }
                Err(nix::errno::Errno::EWOULDBLOCK) => {
                    guard.clear_ready();
                    continue;
                }
                Err(e) => {
                    return Poll::Ready(Err(std::io::Error::from_raw_os_error(e as i32)));
                }
            }
        }
    }
}

impl AsyncWrite for Connection {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        loop {
            let mut guard = match self.conn.poll_write_ready(cx) {
                Poll::Ready(Ok(guard)) => guard,
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => return Poll::Pending,
            };

            let fd = self.conn.get_ref().as_fd();

            match nix::unistd::write(fd, buf) {
                Ok(n) => {
                    return Poll::Ready(Ok(n));
                }
                Err(nix::errno::Errno::EWOULDBLOCK) => {
                    guard.clear_ready();
                    continue;
                }
                Err(e) => {
                    return Poll::Ready(Err(std::io::Error::from_raw_os_error(e as i32)));
                }
            }
        }
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        // For TCP connections, shutdown is typically a no-op or uses the Drop impl
        Poll::Ready(Ok(()))
    }
}

impl Listener {
    /// Accepts a new incoming connection on this listener.
    ///
    /// # Errors
    ///
    /// Returns an error if accepting the connection fails.
    pub async fn accept(self: &Arc<Self>) -> Result<Connection> {
        debug!(fd = self.ln, "waiting to accept connection");
        let ln = self.ln;

        // Use spawn_blocking to run the blocking C call
        let (out_fd, ret) = tokio::task::spawn_blocking(move || {
            let mut out_fd = 0;
            let ret = unsafe { tailscale_accept(ln, &mut out_fd) };
            (out_fd, ret)
        })
        .await
        .map_err(TailscaleError::SpawnBlockingFailed)?;

        if ret != 0 {
            let error_message = self._tailscale.get_error_message()?;
            return Err(TailscaleError::AcceptFailed(error_message));
        }
        debug!(fd = out_fd, "accepted connection");

        // Set the fd to non-blocking mode
        let borrowed_fd = unsafe { std::os::fd::BorrowedFd::borrow_raw(out_fd) };
        let flags = nix::fcntl::OFlag::from_bits_truncate(
            nix::fcntl::fcntl(borrowed_fd, nix::fcntl::FcntlArg::F_GETFL)
                .map_err(|e| TailscaleError::Tailscale(format!("F_GETFL failed: {}", e)))?,
        );
        nix::fcntl::fcntl(
            borrowed_fd,
            nix::fcntl::FcntlArg::F_SETFL(flags | nix::fcntl::OFlag::O_NONBLOCK),
        )
        .map_err(|e| TailscaleError::Tailscale(format!("F_SETFL failed: {}", e)))?;

        // Convert raw fd to OwnedFd and wrap in AsyncFd
        let owned_fd = unsafe { OwnedFd::from_raw_fd(out_fd) };
        let async_fd = AsyncFd::new(owned_fd)
            .map_err(|e| TailscaleError::Tailscale(format!("AsyncFd::new failed: {}", e)))?;

        let listener = Arc::clone(self);

        Ok(Connection {
            conn: async_fd,
            listener: Some(listener),
        })
    }
}

/// A pair of IPv4 and IPv6 addresses assigned to a Tailscale node.
#[derive(Debug)]
pub struct IpPair {
    pub ipv4: Ipv4Addr,
    pub ipv6: Ipv6Addr,
}

/// A Tailscale networking instance.
///
/// This struct represents an active Tailscale node and provides methods
/// for creating listeners and managing the connection.
pub struct Tailscale {
    sd: libc::c_int,
    _log_fd: Option<OwnedFd>,
}

impl Tailscale {
    /// Creates a new builder for configuring a Tailscale instance.
    pub fn builder() -> TailscaleBuilder {
        TailscaleBuilder::default()
    }
    /// Brings up the Tailscale connection.
    ///
    /// This must be called before the Tailscale instance can be used for networking.
    ///
    /// # Errors
    ///
    /// Returns an error if bringing up the connection fails.
    pub async fn up(&self) -> Result<()> {
        debug!("bringing up Tailscale connection");
        let sd = self.sd;

        // Use spawn_blocking for the blocking C call
        let ret = tokio::task::spawn_blocking(move || unsafe { tailscale_up(sd) })
            .await
            .map_err(TailscaleError::SpawnBlockingFailed)?;

        if ret != 0 {
            let error_message = self.get_error_message()?;
            return Err(TailscaleError::UpFailed(error_message));
        }
        debug!("Tailscale connection is up");
        Ok(())
    }

    /// Creates a new listener on the Tailscale network.
    ///
    /// # Arguments
    ///
    /// * `network` - The network type (e.g., `NetworkType::Tcp`)
    /// * `addr` - The address to listen on (e.g., ":8080")
    ///
    /// # Errors
    ///
    /// Returns an error if creating the listener fails.
    pub async fn listener(
        self: &Arc<Tailscale>,
        network: NetworkType,
        addr: &str,
    ) -> Result<Arc<Listener>> {
        debug!(%network, %addr, "creating listener");
        let network_str = network.as_str();
        let network_cstring =
            std::ffi::CString::new(network_str).map_err(TailscaleError::Utf8Error)?;
        let addr_cstring = std::ffi::CString::new(addr).map_err(TailscaleError::Utf8Error)?;
        let sd = self.sd;

        // Use spawn_blocking for the blocking C call
        let (listener, ret) = tokio::task::spawn_blocking(move || {
            let mut listener = 0;
            let ret = unsafe {
                tailscale_listen(
                    sd,
                    network_cstring.as_ptr(),
                    addr_cstring.as_ptr(),
                    &mut listener,
                )
            };
            (listener, ret)
        })
        .await
        .map_err(TailscaleError::SpawnBlockingFailed)?;

        if ret != 0 {
            let error_message = self.get_error_message()?;
            return Err(TailscaleError::ListenFailed {
                network: network_str.to_string(),
                addr: addr.to_string(),
                message: error_message,
            });
        }
        debug!(fd = listener, "listener created");

        Ok(Arc::new(Listener {
            ln: listener,
            _tailscale: Arc::clone(self),
        }))
    }

    /// Creates an outbound connection to another node on the Tailscale network.
    ///
    /// # Arguments
    ///
    /// * `network` - The network type (e.g., `NetworkType::Tcp`)
    /// * `addr` - The address to connect to (e.g., "hostname:8080")
    ///
    /// # Errors
    ///
    /// Returns an error if the connection cannot be established.
    pub async fn connect(&self, network: NetworkType, addr: &str) -> Result<Connection> {
        debug!(%network, %addr, "connecting");
        let network_str = network.as_str();
        let network_cstring =
            std::ffi::CString::new(network_str).map_err(TailscaleError::Utf8Error)?;
        let addr_cstring = std::ffi::CString::new(addr).map_err(TailscaleError::Utf8Error)?;
        let sd = self.sd;

        // Use spawn_blocking for the blocking C call
        let (conn_fd, ret) = tokio::task::spawn_blocking(move || {
            let mut conn_fd = 0;
            let ret = unsafe {
                tailscale_dial(
                    sd,
                    network_cstring.as_ptr(),
                    addr_cstring.as_ptr(),
                    &mut conn_fd,
                )
            };
            (conn_fd, ret)
        })
        .await
        .map_err(TailscaleError::SpawnBlockingFailed)?;

        if ret != 0 {
            let error_message = self.get_error_message()?;
            return Err(TailscaleError::DialFailed {
                network: network_str.to_string(),
                addr: addr.to_string(),
                message: error_message,
            });
        }
        debug!(fd = conn_fd, "connection established");

        // Set the fd to non-blocking mode
        let borrowed_fd = unsafe { std::os::fd::BorrowedFd::borrow_raw(conn_fd) };
        let flags = nix::fcntl::OFlag::from_bits_truncate(
            nix::fcntl::fcntl(borrowed_fd, nix::fcntl::FcntlArg::F_GETFL)
                .map_err(|e| TailscaleError::Tailscale(format!("F_GETFL failed: {}", e)))?,
        );
        nix::fcntl::fcntl(
            borrowed_fd,
            nix::fcntl::FcntlArg::F_SETFL(flags | nix::fcntl::OFlag::O_NONBLOCK),
        )
        .map_err(|e| TailscaleError::Tailscale(format!("F_SETFL failed: {}", e)))?;

        // Convert raw fd to OwnedFd and wrap in AsyncFd
        let owned_fd = unsafe { OwnedFd::from_raw_fd(conn_fd) };
        let async_fd = AsyncFd::new(owned_fd)
            .map_err(|e| TailscaleError::Tailscale(format!("AsyncFd::new failed: {}", e)))?;

        Ok(Connection {
            listener: None,
            conn: async_fd,
        })
    }

    /// Returns the IPv4 and IPv6 addresses assigned to this Tailscale node.
    ///
    /// Returns `None` if no IP addresses have been assigned yet.
    ///
    /// # Errors
    ///
    /// Returns an error if retrieving or parsing the IP addresses fails.
    pub fn ips(&self) -> Result<Option<IpPair>> {
        let buf = [0u8; 256];
        let ret = unsafe { tailscale_getips(self.sd, buf.as_ptr() as *mut _, buf.len()) };
        if ret != 0 {
            let error_message = self.get_error_message()?;
            return Err(TailscaleError::Tailscale(error_message));
        }
        let s = CStr::from_bytes_until_nul(&buf[..])?;
        let s = s.to_str()?;

        if s.is_empty() {
            return Ok(None);
        }

        let (ipv4, ipv6) = s
            .split_once(',')
            .ok_or_else(|| TailscaleError::InvalidIpAdresses(s.to_string()))?;

        let ipv4 = Ipv4Addr::from_str(ipv4)
            .map_err(|e| TailscaleError::AddrParseError(ipv4.to_string(), e))?;
        let ipv6 = Ipv6Addr::from_str(ipv6)
            .map_err(|e| TailscaleError::AddrParseError(ipv6.to_string(), e))?;

        Ok(Some(IpPair { ipv4, ipv6 }))
    }

    fn get_error_message(&self) -> Result<String> {
        let buf = [0u8; 2048];
        let ret = unsafe { tailscale_errmsg(self.sd, buf.as_ptr() as *mut _, buf.len()) };
        if ret > 0 {
            return Err(TailscaleError::Tailscale(format!(
                "Failed to retrieve error message (error code: {})",
                ret
            )));
        }
        let s = CStr::from_bytes_until_nul(&buf[..])?;
        let s = s.to_str()?;
        Ok(s.to_string())
    }
}

impl Drop for Tailscale {
    fn drop(&mut self) {
        debug!("dropping server");
        let ret = unsafe { tailscale_close(self.sd) };
        if ret != 0 {
            if let Ok(error_message) = self.get_error_message() {
                error!(error = %error_message, "error dropping tailscale");
            } else {
                error!("error dropping tailscale (failed to retrieve error message)");
            }
        }
    }
}

impl Drop for Listener {
    fn drop(&mut self) {
        debug!("dropping listener");
        if let Err(e) = nix::unistd::close(self.ln) {
            error!(error = %e, "error closing listener");
        }
    }
}
