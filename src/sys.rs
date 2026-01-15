pub type TailscaleListener = libc::c_int;
pub type TailscaleConn = libc::c_int;

pub mod modern {
    use super::{TailscaleListener, TailscaleConn};

    unsafe extern "C" {
        pub fn tailscale_new() -> libc::c_int;
        pub fn tailscale_start(sd: libc::c_int) -> libc::c_int;
        pub fn tailscale_up(sd: libc::c_int) -> libc::c_int;
        pub fn tailscale_close(sd: libc::c_int) -> libc::c_int;
        pub fn tailscale_set_dir(sd: libc::c_int, dir: *const libc::c_char) -> libc::c_int;
        pub fn tailscale_set_hostname(sd: libc::c_int, hostname: *const libc::c_char) -> libc::c_int;
        pub fn tailscale_set_authkey(sd: libc::c_int, authkey: *const libc::c_char) -> libc::c_int;
        pub fn tailscale_set_control_url(sd: libc::c_int, control_url: *const libc::c_char) -> libc::c_int;
        pub fn tailscale_set_ephemeral(sd: libc::c_int, ephemeral: libc::c_int) -> libc::c_int;
        pub fn tailscale_set_logfd(sd: libc::c_int, fd: libc::c_int) -> libc::c_int;
        pub fn tailscale_getips(sd: libc::c_int, buf: *mut libc::c_char, buflen: libc::size_t) -> libc::c_int;
        pub fn tailscale_dial(
            sd: libc::c_int,
            network: *const libc::c_char,
            addr: *const libc::c_char,
            conn_out: *mut TailscaleConn,
        ) -> libc::c_int;
        pub fn tailscale_listen(
            sd: libc::c_int,
            network: *const libc::c_char,
            addr: *const libc::c_char,
            listener_out: *mut TailscaleListener,
        ) -> libc::c_int;
        pub fn tailscale_getremoteaddr(
            l: TailscaleListener,
            conn: TailscaleConn,
            buf: *mut libc::c_char,
            buflen: libc::size_t,
        ) -> libc::c_int;
        pub fn tailscale_accept(ln: TailscaleListener, conn_out: *mut TailscaleConn) -> libc::c_int;
        pub fn tailscale_loopback(
            sd: libc::c_int,
            addr_out: *mut libc::c_char,
            addrlen: libc::size_t,
            proxy_cred_out: *mut libc::c_char,
            local_api_cred_out: *mut libc::c_char,
        ) -> libc::c_int;
        pub fn tailscale_enable_funnel_to_localhost_plaintext_http1(
            sd: libc::c_int,
            localhost_port: libc::c_int,
        ) -> libc::c_int;
        pub fn tailscale_errmsg(sd: libc::c_int, buf: *mut libc::c_char, buflen: libc::size_t) -> libc::c_int;
    }
}
