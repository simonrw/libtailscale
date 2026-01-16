//! High-level Rust bindings for the Tailscale library.
//!
//! This module provides safe, idiomatic Rust wrappers around the underlying
//! Tailscale C API, enabling easy integration of Tailscale networking into
//! Rust applications.

use std::{
    ffi::{CStr, CString, FromBytesUntilNulError, NulError},
    io::{Read, Write},
    net::{AddrParseError, IpAddr, Ipv4Addr, Ipv6Addr},
    os::fd::BorrowedFd,
    path::PathBuf,
    str::{FromStr, Utf8Error},
    sync::{Arc, Mutex},
    task::Poll,
};

use crate::sys::{TailscaleListener, modern::*};

use thiserror::Error;
use tokio::io::{AsyncRead, AsyncWrite};

/// Errors that can occur when working with Tailscale.
#[derive(Debug, Error)]
pub enum TailscaleError {
    #[error("failed to create Tailscale instance")]
    CreateTailscale,

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

    #[error("tailscale error: {0}")]
    Tailscale(String),
}

/// A specialized `Result` type for Tailscale operations.
pub type Result<T> = std::result::Result<T, TailscaleError>;

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
#[derive(Default, Clone)]
pub struct TailscaleBuilder {
    ephemeral: bool,
    hostname: Option<String>,
    dir: Option<PathBuf>,
    auth_key: Option<String>,
}

impl TailscaleBuilder {
    /// Builds and returns a configured Tailscale instance.
    ///
    /// # Errors
    ///
    /// Returns an error if any of the configuration options fail to be set.
    pub fn build(&self) -> Result<Arc<Tailscale>> {
        let sd = unsafe { tailscale_new() };
        if sd == 0 {
            return Err(TailscaleError::CreateTailscale);
        }

        if self.ephemeral {
            let ret = unsafe { tailscale_set_ephemeral(sd, self.ephemeral as _) };
            if ret != 0 {
                return Err(TailscaleError::SetEphemeral);
            }
        }

        if let Some(path) = &self.dir {
            let path_s = path.display().to_string();
            let path_cs = CString::new(path_s)?;
            let ret = unsafe { tailscale_set_dir(sd, path_cs.as_ptr() as *mut _) };
            if ret != 0 {
                return Err(TailscaleError::SetDir);
            }
        };

        if let Some(hostname) = &self.hostname {
            let c_hostname = CString::new(hostname.clone())?;
            let ret = unsafe { tailscale_set_hostname(sd, c_hostname.as_ptr()) };
            if ret != 0 {
                return Err(TailscaleError::SetHostname);
            }
        }
        if let Some(auth_key) = &self.auth_key {
            let c_auth_key = CString::new(auth_key.clone())?;
            let ret = unsafe { tailscale_set_authkey(sd, c_auth_key.as_ptr()) };
            if ret != 0 {
                return Err(TailscaleError::SetAuthKey);
            }
        }

        Ok(Arc::new(Tailscale { sd }))
    }

    /// Sets the authentication key for this Tailscale instance.
    ///
    /// # Arguments
    ///
    /// * `key` - The Tailscale authentication key
    pub fn auth_key(&mut self, key: impl Into<String>) -> &mut Self {
        let new = self;
        new.auth_key = Some(key.into());
        new
    }

    /// Sets whether this node should be ephemeral.
    ///
    /// Ephemeral nodes are automatically removed from the network when they go offline.
    ///
    /// # Arguments
    ///
    /// * `ephemeral` - Whether the node should be ephemeral
    pub fn ephemeral(&mut self, ephemeral: bool) -> &mut Self {
        let new = self;
        new.ephemeral = ephemeral;
        new
    }

    /// Sets the hostname for this Tailscale node.
    ///
    /// # Arguments
    ///
    /// * `hostname` - The desired hostname for the node
    pub fn hostname(&mut self, hostname: impl Into<String>) -> &mut Self {
        let new = self;
        new.hostname = Some(hostname.into());
        new
    }

    /// Sets the state directory for Tailscale to store its configuration.
    ///
    /// # Arguments
    ///
    /// * `dir` - Path to the directory where Tailscale should store its state
    pub fn dir(&mut self, dir: impl Into<PathBuf>) -> &mut Self {
        let new = self;
        new.dir = Some(dir.into());
        new
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
/// Implements `Read` for reading data from the connection.
#[derive(Clone)]
pub struct Connection {
    listener: Option<Arc<Listener>>,
    // TODO: async mutex?
    conn: Arc<Mutex<TailscaleConn>>,
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

        // TODO: handle poison
        let conn = self.conn.lock().unwrap();
        let buf = [0u8; 128];
        let ret = unsafe {
            tailscale_getremoteaddr(listener.ln, *conn, buf.as_ptr() as *mut _, buf.len())
        };

        // TODO
        if ret > 0 {
            panic!("handle return value");
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
        eprintln!("dropping connection");
        // TODO: handle poison
        let conn = self.conn.lock().unwrap();
        if let Err(e) = nix::unistd::close(*conn) {
            eprintln!("error dropping connection: {e}");
        }
    }
}

impl Read for Connection {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // TODO: handle poison
        let conn = self.conn.lock().unwrap();
        let fd = unsafe { BorrowedFd::borrow_raw(*conn) };
        nix::unistd::read(fd, buf).map_err(|errno| std::io::Error::from_raw_os_error(errno as i32))
    }
}

impl Write for Connection {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // TODO: handle poison
        let conn = self.conn.lock().unwrap();
        let fd = unsafe { BorrowedFd::borrow_raw(*conn) };
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
        todo!()
    }
}

impl AsyncWrite for Connection {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        todo!()
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        todo!()
    }
}

impl Listener {
    /// Accepts a new incoming connection on this listener.
    ///
    /// # Errors
    ///
    /// Returns an error if accepting the connection fails.
    pub async fn accept(self: &Arc<Self>) -> Result<Connection> {
        let mut out_fd = 0;
        let _ret = unsafe { tailscale_accept(self.ln, &mut out_fd) };
        // TODO: handle ret

        let listener = Arc::clone(self);

        Ok(Connection {
            conn: Arc::new(Mutex::new(out_fd)),
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
        let ret = unsafe { tailscale_up(self.sd) };
        self.handle_error(ret)?;
        Ok(())
    }

    /// Creates a new listener on the Tailscale network.
    ///
    /// # Arguments
    ///
    /// * `network` - The network type (e.g., "tcp")
    /// * `addr` - The address to listen on (e.g., ":8080")
    ///
    /// # Errors
    ///
    /// Returns an error if creating the listener fails.
    pub fn listener(
        self: &Arc<Tailscale>,
        network: &str,
        // addr: impl ToSocketAddrs,
        addr: &str,
    ) -> Result<Arc<Listener>> {
        let network = std::ffi::CString::new(network).map_err(TailscaleError::Utf8Error)?;
        let addr = std::ffi::CString::new(addr).map_err(TailscaleError::Utf8Error)?;
        // let addr = addr
        //     .to_socket_addrs()
        //     .map_err(TailscaleError::InvalidAddress)?
        //     .next()
        //     .ok_or_else(|| {
        //         TailscaleError::InvalidAddress(std::io::Error::new(
        //             std::io::ErrorKind::Other,
        //             "invalid address",
        //         ))
        //     })?;

        let mut listener = 0;

        let ret =
            unsafe { tailscale_listen(self.sd, network.as_ptr(), addr.as_ptr(), &mut listener) };
        self.handle_error(ret)?;

        Ok(Arc::new(Listener {
            ln: listener,
            _tailscale: Arc::clone(self),
        }))
    }

    /// Creates an outbound connection to another node on the Tailscale network.
    ///
    /// # Arguments
    ///
    /// * `network` - The network type (e.g., "tcp")
    /// * `addr` - The address to connect to (e.g., "hostname:8080")
    ///
    /// # Errors
    ///
    /// Returns an error if the connection cannot be established.
    pub fn connect<'t, 's: 't>(&'t self, network: &str, addr: &str) -> Result<Arc<Connection>> {
        let network = std::ffi::CString::new(network).map_err(TailscaleError::Utf8Error)?;
        let addr = std::ffi::CString::new(addr).map_err(TailscaleError::Utf8Error)?;
        let mut conn_fd = 0;

        let ret = unsafe { tailscale_dial(self.sd, network.as_ptr(), addr.as_ptr(), &mut conn_fd) };
        self.handle_error(ret)?;

        Ok(Arc::new(Connection {
            listener: None,
            conn: Arc::new(Mutex::new(conn_fd)),
        }))
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
        self.handle_error(ret)?;
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

    fn handle_error(&self, value: libc::c_int) -> Result<()> {
        if value > 0 {
            let error_message = self.get_error_message()?;
            return Err(TailscaleError::Tailscale(error_message));
        }
        Ok(())
    }

    fn get_error_message(&self) -> Result<String> {
        let buf = [0u8; 2048];
        let ret = unsafe { tailscale_errmsg(self.sd, buf.as_ptr() as *mut _, buf.len()) };
        if ret > 0 {
            todo!("error with fetching error message: {ret}")
        }
        let s = CStr::from_bytes_until_nul(&buf[..])?;
        let s = s.to_str()?;
        Ok(s.to_string())
    }
}

impl Drop for Tailscale {
    fn drop(&mut self) {
        eprintln!("dropping server");
        let ret = unsafe { tailscale_close(self.sd) };
        if let Err(e) = self.handle_error(ret) {
            eprintln!("error dropping tailscale: {e}");
        }
    }
}

impl Drop for Listener {
    fn drop(&mut self) {
        eprintln!("dropping listener");
        if let Err(e) = nix::unistd::close(self.ln) {
            eprintln!("Error closing listener: {e}");
        }
    }
}
