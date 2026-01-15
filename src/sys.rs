pub type TailscaleListener = libc::c_int;
pub type TailscaleConn = libc::c_int;

pub mod modern {
    use super::{TailscaleListener, TailscaleConn};

    unsafe extern "C" {
        /// Creates a tailscale server object.
        ///
        /// No network connection is initialized until tailscale_start is called.
        pub fn tailscale_new() -> libc::c_int;

        /// Connects the server to the tailnet.
        ///
        /// Calling this function is optional as it will be called by the first use
        /// of tailscale_listen or tailscale_dial on a server.
        ///
        /// See also: tailscale_up.
        ///
        /// Returns zero on success or -1 on error, call tailscale_errmsg for details.
        // pub fn tailscale_start(sd: libc::c_int) -> libc::c_int;

        /// Connects the server to the tailnet and waits for it to be usable.
        ///
        /// To cancel an in-progress call to tailscale_up, use tailscale_close.
        ///
        /// Returns zero on success or -1 on error, call tailscale_errmsg for details.
        pub fn tailscale_up(sd: libc::c_int) -> libc::c_int;

        /// Shuts down the server.
        ///
        /// Returns:
        /// - 0     - success
        /// - EBADF - sd is not a valid tailscale
        /// - -1    - other error, details printed to the tsnet logger
        pub fn tailscale_close(sd: libc::c_int) -> libc::c_int;

        /// Sets the directory for tailscale state storage.
        ///
        /// Configure this option before any explicit or implicit call to tailscale_start.
        ///
        /// For details of each value see the godoc for the fields of tsnet.Server.
        ///
        /// Returns zero on success or -1 on error, call tailscale_errmsg for details.
        pub fn tailscale_set_dir(sd: libc::c_int, dir: *const libc::c_char) -> libc::c_int;

        /// Sets the hostname for the tailscale node.
        ///
        /// Configure this option before any explicit or implicit call to tailscale_start.
        ///
        /// For details of each value see the godoc for the fields of tsnet.Server.
        ///
        /// Returns zero on success or -1 on error, call tailscale_errmsg for details.
        pub fn tailscale_set_hostname(sd: libc::c_int, hostname: *const libc::c_char) -> libc::c_int;

        /// Sets the authentication key.
        ///
        /// Configure this option before any explicit or implicit call to tailscale_start.
        ///
        /// For details of each value see the godoc for the fields of tsnet.Server.
        ///
        /// Returns zero on success or -1 on error, call tailscale_errmsg for details.
        pub fn tailscale_set_authkey(sd: libc::c_int, authkey: *const libc::c_char) -> libc::c_int;

        /// Sets the control URL.
        ///
        /// Configure this option before any explicit or implicit call to tailscale_start.
        ///
        /// For details of each value see the godoc for the fields of tsnet.Server.
        ///
        /// Returns zero on success or -1 on error, call tailscale_errmsg for details.
        pub fn tailscale_set_control_url(sd: libc::c_int, control_url: *const libc::c_char) -> libc::c_int;

        /// Sets whether the node is ephemeral.
        ///
        /// Configure this option before any explicit or implicit call to tailscale_start.
        ///
        /// For details of each value see the godoc for the fields of tsnet.Server.
        ///
        /// Returns zero on success or -1 on error, call tailscale_errmsg for details.
        pub fn tailscale_set_ephemeral(sd: libc::c_int, ephemeral: libc::c_int) -> libc::c_int;

        /// Instructs the tailscale instance to write logs to fd.
        ///
        /// An fd value of -1 means discard all logging.
        ///
        /// Returns zero on success or -1 on error, call tailscale_errmsg for details.
        pub fn tailscale_set_logfd(sd: libc::c_int, fd: libc::c_int) -> libc::c_int;

        /// Returns the IP addresses of the Tailscale server as a comma separated list.
        ///
        /// The provided buffer must be of sufficient size to hold the concatenated
        /// IPs as strings. This is typically <ipv4>,<ipv6> but maybe empty, or
        /// contain any number of ips. The caller is responsible for parsing
        /// the output. You may assume the output is a list of well-formed IPs.
        ///
        /// Returns:
        /// - 0      - Success
        /// - EBADF  - sd is not a valid tailscale, or l or conn are not valid listeners or connections
        /// - ERANGE - insufficient storage for buf
        pub fn tailscale_getips(sd: libc::c_int, buf: *mut libc::c_char, buflen: libc::size_t) -> libc::c_int;

        /// Connects to the address on the tailnet.
        ///
        /// The newly allocated connection is written to conn_out.
        ///
        /// network is a NUL-terminated string of the form "tcp", "udp", etc.
        /// addr is a NUL-terminated string of an IP address or domain name.
        ///
        /// It will start the server if it has not been started yet.
        ///
        /// Returns zero on success or -1 on error, call tailscale_errmsg for details.
        pub fn tailscale_dial(
            sd: libc::c_int,
            network: *const libc::c_char,
            addr: *const libc::c_char,
            conn_out: *mut TailscaleConn,
        ) -> libc::c_int;

        /// Listens for a connection on the tailnet.
        ///
        /// It is the spiritual equivalent to listen(2).
        /// The newly allocated listener is written to listener_out.
        ///
        /// network is a NUL-terminated string of the form "tcp", "udp", etc.
        /// addr is a NUL-terminated string of an IP address or domain name.
        ///
        /// It will start the server if it has not been started yet.
        ///
        /// Returns zero on success or -1 on error, call tailscale_errmsg for details.
        pub fn tailscale_listen(
            sd: libc::c_int,
            network: *const libc::c_char,
            addr: *const libc::c_char,
            listener_out: *mut TailscaleListener,
        ) -> libc::c_int;

        /// Returns the remote address for an incoming connection for a particular listener.
        ///
        /// The address (either ip4 or ip6) will be written to buf on success.
        ///
        /// Returns:
        /// - 0      - Success
        /// - EBADF  - sd is not a valid tailscale, or l or conn are not valid listeners or connections
        /// - ERANGE - insufficient storage for buf
        pub fn tailscale_getremoteaddr(
            l: TailscaleListener,
            conn: TailscaleConn,
            buf: *mut libc::c_char,
            buflen: libc::size_t,
        ) -> libc::c_int;

        /// Accepts a connection on a tailscale_listener.
        ///
        /// It is the spiritual equivalent to accept(2).
        ///
        /// The newly allocated connection is written to conn_out.
        ///
        /// Returns:
        /// - 0     - success
        /// - EBADF - listener is not a valid tailscale
        /// - -1    - call tailscale_errmsg for details
        pub fn tailscale_accept(ln: TailscaleListener, conn_out: *mut TailscaleConn) -> libc::c_int;

        /// Starts a loopback address server.
        ///
        /// The server has multiple functions.
        ///
        /// It can be used as a SOCKS5 proxy onto the tailnet.
        /// Authentication is required with the username "tsnet" and
        /// the value of proxy_cred used as the password.
        ///
        /// The HTTP server also serves out the "LocalAPI" on /localapi.
        /// As the LocalAPI is powerful, access to endpoints requires BOTH passing a
        /// "Sec-Tailscale: localapi" HTTP header and passing local_api_cred as
        /// the basic auth password.
        ///
        /// The pointers proxy_cred_out and local_api_cred_out must be non-NIL
        /// and point to arrays that can hold 33 bytes. The first 32 bytes are
        /// the credential and the final byte is a NUL terminator.
        ///
        /// If tailscale_loopback returns, then addr_out, proxy_cred_out,
        /// and local_api_cred_out are all NUL-terminated.
        ///
        /// Returns zero on success or -1 on error, call tailscale_errmsg for details.
        pub fn tailscale_loopback(
            sd: libc::c_int,
            addr_out: *mut libc::c_char,
            addrlen: libc::size_t,
            proxy_cred_out: *mut libc::c_char,
            local_api_cred_out: *mut libc::c_char,
        ) -> libc::c_int;

        /// Configures sd to have Tailscale Funnel enabled.
        ///
        /// Routes requests from the public web (without any authentication) down to this
        /// Tailscale node, requesting new LetsEncrypt TLS certs as needed, terminating TLS,
        /// and proxying all incoming HTTPS requests to http://127.0.0.1:localhostPort without TLS.
        ///
        /// There should be a plaintext HTTP/1 server listening on 127.0.0.1:localhostPort
        /// or tsnet will serve HTTP 502 errors.
        ///
        /// Expect junk traffic from the internet from bots watching the public CT logs.
        ///
        /// Returns:
        /// - 0  - success
        /// - -1 - other error, details printed to the tsnet logger
        pub fn tailscale_enable_funnel_to_localhost_plaintext_http1(
            sd: libc::c_int,
            localhost_port: libc::c_int,
        ) -> libc::c_int;

        /// Writes the details of the last error to buf.
        ///
        /// After returning, buf is always NUL-terminated.
        ///
        /// Returns:
        /// - 0      - success
        /// - EBADF  - sd is not a valid tailscale
        /// - ERANGE - insufficient storage for buf
        pub fn tailscale_errmsg(sd: libc::c_int, buf: *mut libc::c_char, buflen: libc::size_t) -> libc::c_int;
    }
}
