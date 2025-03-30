// Copyright (c) Tailscale Inc & AUTHORS
// SPDX-License-Identifier: BSD-3-Clause

#include <pybind11/pybind11.h>
#include "../tailscale.h"

#define STRINGIFY(x) #x
#define MACRO_STRINGIFY(x) STRINGIFY(x)

namespace py = pybind11;

PYBIND11_MODULE(_tailscale, m) {
    m.doc() = R"pbdoc(
        Embedded Tailscale
        -----------------------

        .. currentmodule:: _tailscale

        .. autosummary::
           :toctree: _generate
    )pbdoc";

    m.def("new", &tailscale_new, R"pbdoc(
        Create a new tsnet server
    )pbdoc");

    m.def("start", &tailscale_start, R"pbdoc(
        Starts a tsnet server
    )pbdoc");

    m.def("up", &tailscale_up, R"pbdoc(
        Brings the given tsnet server up
    )pbdoc");

    m.def("close", &tailscale_close, R"pbdoc(
        Closes a given tsnet server
    )pbdoc");

    m.def("err_msg", &tailscale_errmsg, R"pbdoc(

    )pbdoc");

    m.def("listen", [](int sd, char* network, char* addr) {
            int listenerOut;
            int rv = tailscale_listen(sd, network, addr, &listenerOut);
            return std::make_tuple(listenerOut, rv);
            }, R"pbdoc(
        Listen on a given protocol and port
    )pbdoc");

    m.def("accept", [](int ld) {
            int connOut;
            int rv = tailscale_accept(ld, &connOut);
            return std::make_tuple(connOut, rv);
    }, R"pbdoc(
        Accept a given listener and connection
    )pbdoc");

    m.def("dial", &tailscale_dial, R"pbdoc(

    )pbdoc");

    m.def("set_dir", &tailscale_set_dir, R"pbdoc(

    )pbdoc");

    m.def("set_hostname", &tailscale_set_hostname, R"pbdoc(

    )pbdoc");

    m.def("set_authkey", &tailscale_set_authkey, R"pbdoc(

    )pbdoc");

    m.def("set_control_url", &tailscale_set_control_url, R"pbdoc(

    )pbdoc");

    m.def("set_ephemeral", &tailscale_set_ephemeral, R"pbdoc(
        Set the given tsnet server to be an ephemeral node.
    )pbdoc");

    m.def("set_log_fd", &tailscale_set_logfd, R"pbdoc(

    )pbdoc");

    m.def("loopback", &tailscale_loopback, R"pbdoc(

    )pbdoc");

#ifdef VERSION_INFO
    m.attr("__version__") = MACRO_STRINGIFY(VERSION_INFO);
#else
    m.attr("__version__") = "dev";
#endif
}
