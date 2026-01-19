# libtailscale Rust Bindings

This document provides documentation for the Rust bindings for libtailscale, located in the `src/` directory.

## Overview

The Rust codebase provides safe, idiomatic Rust bindings for the Tailscale networking library, enabling integration of Tailscale's mesh VPN capabilities into Rust applications. The bindings wrap the underlying C API from libtailscale.a, providing:

- Type-safe, async/await-based API
- Structured error handling with thiserror
- Tracing support for structured logging
- Tokio-based async I/O for network operations
- Builder pattern for configuration

## Project Structure

The Rust codebase is organized into three main modules under `src/`:

```
src/
├── lib.rs          # Public API and crate-level documentation
├── tailscale.rs    # High-level Rust bindings and types
└── sys.rs          # Low-level C FFI declarations
```

### Module Descriptions

- **`lib.rs`**: Entry point for the crate. Contains top-level documentation, usage examples, and re-exports the public API.

- **`tailscale.rs`**: Core implementation containing:
  - `Tailscale` struct: Main interface for creating and managing Tailscale instances
  - `TailscaleBuilder`: Builder pattern for configuring Tailscale connections
  - `Listener`: TCP listener on the Tailscale network
  - `Connection`: Accepted connection with async I/O traits
  - `TailscaleError`: Comprehensive error types
  - `LogConfig`: Logging configuration options

- **`sys.rs`**: Foreign Function Interface (FFI) declarations for the C API from libtailscale. Contains unsafe extern "C" function declarations that wrap the underlying Go implementation.

## Building

### Building the Static Library

The project builds `libtailscale2.a`, a static library that combines the Go-based libtailscale.a with additional C bindings:

```bash
make build
```

This command:
1. Builds `libtailscale.a` from the Go source using `go build -buildmode=c-archive`
2. Compiles `tailscale.c` into `scratch/tailscale.o`
3. Extracts object files from `libtailscale.a`
4. Combines all object files into `libtailscale2.a` using `ar`

The resulting `libtailscale2.a` file is linked into the Rust project during compilation.

### Build Process Details

The Rust build process (defined in `build.rs`) instructs the compiler to:
- Link against the static library: `libtailscale2.a`
- Search for libraries in the current directory
- On macOS, link additional system frameworks:
  - CoreFoundation
  - IOKit
  - Security

### Building Rust Examples

After building the static library, you can build Rust examples:

```bash
cargo build --examples
```

Run an example:

```bash
cargo run --example echo
```

## Dependencies

The project uses the following Rust dependencies (from `Cargo.toml`):

### Production Dependencies

| Dependency | Version | Features | Purpose |
|------------|---------|----------|---------|
| **libc** | 0.2.180 | (default) | Low-level C type definitions for FFI |
| **nix** | 0.30.1 | socket, uio, fs | Unix system call wrappers for socket operations and file descriptors |
| **thiserror** | 2.0.17 | (default) | Derive macro for error types, provides clean error definitions |
| **tokio** | 1.49.0 | io-util, net, rt | Async runtime for non-blocking I/O operations |
| **tracing** | 0.1 | (default) | Structured logging and diagnostics |

### Development Dependencies

| Dependency | Version | Features | Purpose |
|------------|---------|----------|---------|
| **tokio** | 1.49.0 | full | Complete tokio features for examples and tests |
| **tracing-subscriber** | 0.3 | env-filter | Log collection and filtering for examples |

### Dependency Details

- **libc**: Provides raw C types (`c_int`, `c_char`) for FFI boundaries
- **nix**: Safe wrappers around Unix system calls, used for:
  - Socket operations (read/write on file descriptors)
  - Setting file descriptors to non-blocking mode
  - File control operations (fcntl)
- **thiserror**: Simplifies error handling with the `#[derive(Error)]` macro
- **tokio**: Async runtime powering:
  - `AsyncRead` and `AsyncWrite` traits for Connection
  - `spawn_blocking` for calling blocking C functions
  - `AsyncFd` for async operations on file descriptors
- **tracing**: Structured logging throughout the library for debugging and observability

## API Overview

### Creating a Tailscale Instance

```rust
use tailscale2::Tailscale;

let ts = Tailscale::builder()
    .hostname("my-node")
    .ephemeral(true)
    .dir("/path/to/state")
    .auth_key("tskey-...")
    .build()?;
```

### Configuration Options

The `TailscaleBuilder` supports:

- **`hostname(name)`**: Sets the node's hostname on the tailnet
- **`ephemeral(bool)`**: Makes the node ephemeral (auto-cleanup when offline)
- **`dir(path)`**: Sets the state directory for persistent configuration
- **`auth_key(key)`**: Sets the authentication key for automatic login
- **`log_destination(fd)`**: Redirects Tailscale logs to a custom file descriptor
- **`log_discard()`**: Disables all Tailscale logging

### Establishing Connection

```rust
ts.up().await?;
```

Brings up the Tailscale connection. This is an async operation that blocks until the node is connected to the tailnet.

### Creating a Listener

```rust
let listener = ts.listener("tcp", ":8080").await?;
```

Creates a TCP listener on port 8080. The listener accepts connections from other nodes on the tailnet.

### Accepting Connections

```rust
loop {
    let conn = listener.accept().await?;
    tokio::spawn(async move {
        // Handle connection
    });
}
```

### Connecting to Remote Nodes

```rust
let conn = ts.connect("tcp", "hostname:8080").await?;
```

### Working with Connections

Connections implement both sync and async I/O traits:

```rust
// Async I/O with tokio
use tokio::io::{AsyncReadExt, AsyncWriteExt};
let n = conn.read(&mut buf).await?;
conn.write_all(b"hello").await?;

// Sync I/O with std
use std::io::{Read, Write};
let n = conn.read(&mut buf)?;
conn.write_all(b"hello")?;
```

### Retrieving Node IP Addresses

```rust
if let Some(ips) = ts.ips()? {
    println!("IPv4: {}", ips.ipv4);
    println!("IPv6: {}", ips.ipv6);
}
```

### Logging Configuration

```rust
use std::fs::File;

// Write to a file
let log_file = File::create("/tmp/tailscale.log")?;
let ts = Tailscale::builder()
    .log_destination(log_file)
    .build()?;

// Disable logging
let ts = Tailscale::builder()
    .log_discard()
    .build()?;
```

## Error Handling

All operations return `Result<T, TailscaleError>`. The `TailscaleError` enum covers:

- `CreateTailscale`: Failed to create instance
- `SpawnBlockingFailed`: Background task failed
- `AddrParseError`: Invalid address format
- `Utf8Error`: String encoding issues
- `InvalidAddress`: Invalid listen/dial address
- `SetHostname/SetDir/SetAuthKey/SetEphemeral/SetLogFd`: Configuration errors
- `Tailscale(String)`: Errors from the underlying C/Go library

## Examples

The `examples/` directory contains practical demonstrations:

### Echo Server (`examples/echo.rs`)

A TCP echo server that accepts connections and echoes back received data:

```bash
cargo run --example echo
```

Key features demonstrated:
- Creating a Tailscale instance with builder pattern
- Calling `up()` to establish connection
- Creating a listener
- Accepting connections in a loop
- Spawning tasks to handle connections concurrently
- Using async I/O traits

### Client (`examples/client.rs`)

Demonstrates connecting to a remote Tailscale node.

### Logging Demo (`examples/logging_demo.rs`)

Shows how to configure custom log destinations.

## Architecture Notes

### Async Design

The library uses `tokio::task::spawn_blocking` to wrap blocking C API calls, ensuring they don't block the async runtime. File descriptors returned from C are set to non-blocking mode and wrapped in `tokio::io::AsyncFd` for async operations.

### Memory Management

- `Tailscale` implements `Drop` to call `tailscale_close()`
- `Listener` implements `Drop` to close the listener file descriptor
- `Connection` uses `OwnedFd` wrapped in `AsyncFd` for automatic cleanup
- Log file descriptors are owned by the `Tailscale` instance via `_log_fd` field

### Thread Safety

The `Tailscale` struct is wrapped in `Arc` (atomic reference counting) to enable sharing across async tasks. Internal state is managed by the underlying C/Go implementation.

## Edition and Toolchain

The project uses Rust edition 2024, requiring a recent Rust toolchain with support for this edition.

## Testing

Run tests with:

```bash
cargo test
```

Note: Some tests require an active Tailscale account and may need authentication.

## Development Workflow

1. Build the C archive: `make build`
2. Build Rust code: `cargo build`
3. Run examples: `cargo run --example echo`
4. Run with logging: `RUST_LOG=debug cargo run --example echo`

## Tracing and Debugging

Enable tracing output with the `RUST_LOG` environment variable:

```bash
RUST_LOG=tailscale2=debug cargo run --example echo
```

The library uses structured logging with the `tracing` crate, logging operations such as:
- Instance creation
- Configuration changes
- Connection establishment
- Listener creation
- Connection acceptance
- Error conditions

## Platform Support

The library is primarily tested on:
- Linux (primary platform)
- macOS (with additional framework dependencies)

iOS builds are supported via separate Makefile targets but require the iOS SDK.
