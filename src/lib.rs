//! Rust bindings for libtailscale.
//!
//! This crate provides safe, idiomatic Rust bindings for the Tailscale networking library,
//! enabling easy integration of Tailscale's mesh VPN capabilities into Rust applications.
//!
//! # Features
//!
//! - Create and manage Tailscale network instances
//! - Listen for and accept connections on the Tailscale network
//! - Configure nodes with hostnames, authentication keys, and state directories
//! - Support for ephemeral nodes that automatically clean up when disconnected
//!
//! # Example: Echo Server
//!
//! This example creates a simple TCP echo server that listens on the Tailscale network:
//!
//! ```no_run
//! use std::io::Read;
//! use tailscale2::*;
//!
//! fn handle_connection(mut conn: Connection) {
//!     let mut buf = [0u8; 2048];
//!     loop {
//!         let i = conn.read(&mut buf).unwrap();
//!         if i == 0 {
//!             eprintln!("connection dropped");
//!             break;
//!         }
//!
//!         if let Ok(value) = std::str::from_utf8(&buf[..i]) {
//!             println!("{}", value.trim());
//!         }
//!     }
//! }
//!
//! fn main() {
//!     // Create and configure a Tailscale instance
//!     let ts = Tailscale::builder()
//!         .ephemeral(true)
//!         .hostname("foo")
//!         .build()
//!         .unwrap();
//!
//!     // Bring up the Tailscale connection
//!     ts.up().unwrap();
//!
//!     // Create a TCP listener on port 1999
//!     let listener = ts.listener("tcp", ":1999").unwrap();
//!     eprintln!("listening for connections");
//!
//!     // Accept and handle connections
//!     std::thread::scope(|s| {
//!         loop {
//!             let conn = listener.accept().unwrap();
//!             eprintln!("got connection from {}", conn.remote_addr().unwrap());
//!             s.spawn(move || handle_connection(conn));
//!         }
//!     })
//! }
//! ```
//!
//! # Basic Usage
//!
//! 1. Create a Tailscale instance using the builder pattern
//! 2. Call `up()` to establish the connection
//! 3. Create listeners or dialers as needed
//! 4. Handle connections using standard Rust I/O traits

pub use tailscale::*;
mod sys;
mod tailscale;
