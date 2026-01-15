use std::{
    ffi::{CString, NulError},
    io::Read,
    os::fd::BorrowedFd,
    path::PathBuf,
};

use crate::sys::{TailscaleListener, modern::*};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum TailscaleError {
    #[error("invalid utf-8 string")]
    Utf8Error(#[from] NulError),

    #[error("invalid listen address given")]
    InvalidAddress(#[from] std::io::Error),

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
}

pub type Result<T> = std::result::Result<T, TailscaleError>;

pub struct Tailscale {
    sd: libc::c_int,
}

#[derive(Default, Clone)]
pub struct TailscaleBuilder {
    ephemeral: bool,
    hostname: Option<String>,
    dir: Option<PathBuf>,
    auth_key: Option<String>,
}

impl TailscaleBuilder {
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

    pub fn auth_key(&mut self, key: impl Into<String>) -> &mut Self {
        let new = self;
        new.auth_key = Some(key.into());
        new
    }

    pub fn ephemeral(&mut self, ephemeral: bool) -> &mut Self {
        let new = self;
        new.ephemeral = ephemeral;
        new
    }

    pub fn hostname(&mut self, hostname: impl Into<String>) -> &mut Self {
        let new = self;
        new.hostname = Some(hostname.into());
        new
    }
    pub fn dir(&mut self, dir: impl Into<PathBuf>) -> &mut Self {
        let new = self;
        new.dir = Some(dir.into());
        new
    }
}

pub struct Listener<'t> {
    ln: TailscaleListener,
    _tailscale: &'t Tailscale,
}

pub type TailscaleConn = libc::c_int;

pub struct Connection {
    conn: TailscaleConn,
}

impl Drop for Connection {
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

impl Read for Connection {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let fd = unsafe { BorrowedFd::borrow_raw(self.conn) };
        nix::unistd::read(fd, buf).map_err(|errno| std::io::Error::from_raw_os_error(errno as i32))
    }
}

impl<'t> Listener<'t> {
    pub fn accept(&self) -> Result<Connection> {
        let mut out_fd = 0;
        let _ret = unsafe { tailscale_accept(self.ln, &mut out_fd) };
        // TODO: handle ret
        Ok(Connection { conn: out_fd })
    }
}

impl Tailscale {
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

    pub fn up(&self) -> Result<()> {
        let ret = unsafe { tailscale_up(self.sd) };
        self.handle_error(ret)?;
        Ok(())
    }

    pub fn listener(
        &self,
        network: &str,
        // addr: impl ToSocketAddrs,
        addr: &str,
    ) -> Result<Listener<'_>> {
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

    fn handle_error(&self, value: libc::c_int) -> Result<()> {
        if value > 0 {
            panic!("Up bad: {value}");
        }
        Ok(())
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
