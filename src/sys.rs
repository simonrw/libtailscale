pub type TailscaleListener = libc::c_int;

pub mod modern {
    use super::TailscaleListener;

    unsafe extern "C" {
        pub fn tailscale_new() -> libc::c_int;
        pub fn tailscale_up(sd: libc::c_int) -> libc::c_int;
        pub fn tailscale_listen(
            sd: libc::c_int,
            network: *const libc::c_char,
            addr: *const libc::c_char,
            listener_out: *mut TailscaleListener,
        ) -> libc::c_int;
        pub fn tailscale_close(sd: libc::c_int) -> libc::c_int;
        pub fn tailscale_accept(ln: TailscaleListener, conn_out: *mut libc::c_int) -> libc::c_int;
    }
}
