pub type TailscaleListener = libc::c_int;

unsafe extern "C" {
    pub fn TsnetNewServer() -> libc::c_int;
    pub fn TsnetStart(sd: libc::c_int) -> libc::c_int;
    pub fn TsnetUp(sd: libc::c_int) -> libc::c_int;
    pub fn TsnetClose(sd: libc::c_int) -> libc::c_int;
    pub fn TsnetGetIps(
        sd: libc::c_int,
        buf: *mut libc::c_char,
        buflen: libc::size_t,
    ) -> libc::c_int;
    pub fn TsnetErrmsg(
        sd: libc::c_int,
        buf: *mut libc::c_char,
        buflen: libc::size_t,
    ) -> libc::c_int;
    pub fn TsnetListen(
        sd: libc::c_int,
        network: *const libc::c_char,
        addr: *const libc::c_char,
        listener_out: *mut TailscaleListener,
    ) -> libc::c_int;
    pub fn TsnetGetRemoteAddr(
        listener: libc::c_int,
        conn: libc::c_int,
        buf: *mut libc::c_char,
        buflen: libc::size_t,
    ) -> libc::c_int;
    pub fn TsnetDial(
        sd: libc::c_int,
        network: libc::c_char,
        addr: libc::c_char,
        conn_out: *mut libc::c_int,
    ) -> libc::c_int;
    pub fn TsnetSetDir(sd: libc::c_int, str: *mut libc::c_char) -> libc::c_int;
    pub fn TsnetSetHostname(sd: libc::c_int, str: *mut libc::c_char) -> libc::c_int;
    pub fn TsnetSetAuthKey(sd: libc::c_int, str: *mut libc::c_char) -> libc::c_int;
    pub fn TsnetSetControlURL(sd: libc::c_int, str: *mut libc::c_char) -> libc::c_int;
    pub fn TsnetSetEphemeral(sd: libc::c_int, e: libc::c_int) -> libc::c_int;
    pub fn TsnetSetLogFD(sd: libc::c_int, fd: libc::c_int) -> libc::c_int;
    pub fn TsnetLoopback(
        sd: libc::c_int,
        addrOut: libc::c_char,
        addrLen: libc::size_t,
        proxyOut: libc::c_char,
        localOut: libc::c_char,
    ) -> libc::c_int;
    pub fn TsnetEnableFunnelToLocalhostPlaintextHttp1(
        sd: libc::c_int,
        localhost_port: libc::c_int,
    ) -> libc::c_int;
}
