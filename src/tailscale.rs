//! High-level Rust bindings for the Tailscale library.
//!
//! This module provides safe, idiomatic Rust wrappers around the underlying
//! Tailscale C API, enabling easy integration of Tailscale networking into
//! Rust applications.

use std::{
    ffi::{CStr, CString, FromBytesUntilNulError, NulError},
    io::Read,
    net::{AddrParseError, IpAddr, Ipv4Addr, Ipv6Addr},
    os::fd::BorrowedFd,
    path::PathBuf,
    str::{FromStr, Utf8Error},
};

use crate::sys::{TailscaleListener, modern::*};

use thiserror::Error;

/// Errors that can occur when working with Tailscale.
#[derive(Debug, Error)]
pub enum TailscaleError {
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
/// # use libtailscale::Tailscale;
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
    pub fn build(&self) -> Result<Tailscale> {
        let sd = unsafe { tailscale_new() };
        // TODO: handle if sd is 0
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

        Ok(Tailscale { sd })
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
pub struct Listener<'t> {
    ln: TailscaleListener,
    _tailscale: &'t Tailscale,
}

pub type TailscaleConn = libc::c_int;

/// A connection accepted from a Tailscale listener.
///
/// Implements `Read` for reading data from the connection.
pub struct Connection<'t, 's: 't> {
    listener: &'t Listener<'s>,
    conn: TailscaleConn,
}

impl<'t, 's> Connection<'t, 's> {
    /// Returns the remote IP address of this connection.
    ///
    /// # Errors
    ///
    /// Returns an error if the remote address cannot be retrieved or parsed.
    pub fn remote_addr(&self) -> Result<IpAddr> {
        let buf = [0u8; 128];
        let ret = unsafe {
            tailscale_getremoteaddr(
                self.listener.ln,
                self.conn,
                buf.as_ptr() as *mut _,
                buf.len(),
            )
        };

        // TODO
        if ret > 0 {
            panic!("handle return value");
        }

        let s = CStr::from_bytes_until_nul(&buf[..])?;
        let s = s.to_str()?;

        let addr =
            IpAddr::from_str(s).map_err(|e| TailscaleError::AddrParseError(s.to_string(), e))?;

        Ok(addr)
    }
}

impl<'t, 's> Drop for Connection<'t, 's> {
    #[cfg(unix)]
    fn drop(&mut self) {
        eprintln!("dropping connection");
        if let Err(e) = nix::unistd::close(self.conn) {
            eprintln!("error dropping connection: {e}");
        }
    }

    #[cfg(not(unix))]
    fn drop(&mut self) {
        // TODO
    }
}

impl<'t, 's> Read for Connection<'t, 's> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let fd = unsafe { BorrowedFd::borrow_raw(self.conn) };
        nix::unistd::read(fd, buf).map_err(|errno| std::io::Error::from_raw_os_error(errno as i32))
    }
}

impl<'t> Listener<'t> {
    /// Accepts a new incoming connection on this listener.
    ///
    /// # Errors
    ///
    /// Returns an error if accepting the connection fails.
    pub fn accept(&self) -> Result<Connection<'t, '_>> {
        let mut out_fd = 0;
        let _ret = unsafe { tailscale_accept(self.ln, &mut out_fd) };
        // TODO: handle ret
        Ok(Connection {
            conn: out_fd,
            listener: self,
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
    // pub fn new() -> Result<Self> {
    //     let dir = CString::new("/tmp")?;
    //     let sd = unsafe { TsnetNewServer() };
    //
    //     // TODO: handle if sd is 0
    //     let ret = unsafe { TsnetSetDir(sd, dir.as_ptr() as *mut _) };
    //     if ret != 0 {
    //         panic!("bad");
    //     }
    //
    //     Ok(Self { sd })
    // }

    // pub fn ephemeral() -> Result<Self> {
    //     let me = Self::new()?;
    //     let ret = unsafe { TsnetSetEphemeral(me.sd, 1) };
    //     me.handle_error(ret)?;
    //     Ok(me)
    // }

    /// Brings up the Tailscale connection.
    ///
    /// This must be called before the Tailscale instance can be used for networking.
    ///
    /// # Errors
    ///
    /// Returns an error if bringing up the connection fails.
    pub fn up(&self) -> Result<()> {
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
    pub fn listener<'t>(
        &'t self,
        network: &str,
        // addr: impl ToSocketAddrs,
        addr: &str,
    ) -> Result<Listener<'t>> {
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

        Ok(Listener {
            ln: listener,
            _tailscale: self,
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

impl<'t> Drop for Listener<'t> {
    #[cfg(unix)]
    fn drop(&mut self) {
        eprintln!("dropping listener");
        if let Err(e) = nix::unistd::close(self.ln) {
            eprintln!("Error closing listener: {e}");
        }
    }

    #[cfg(not(unix))]
    fn drop(&mut self) {
        // TODO
    }
}
