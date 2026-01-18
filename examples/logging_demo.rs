use std::fs::File;
use tailscale2::*;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("=== Tailscale Logging Configuration Demo ===\n");

    // Example 1: Default logging (no configuration)
    println!("1. Creating Tailscale with default logging...");
    let ts1 = Tailscale::builder()
        .hostname("demo-default")
        .ephemeral(true)
        .build()?;
    println!("   ✓ Default logging configured\n");
    drop(ts1);

    // Example 2: Log to a file
    println!("2. Creating Tailscale with file logging...");
    let log_file = File::create("/tmp/tailscale_demo.log")?;
    let ts2 = Tailscale::builder()
        .hostname("demo-file-log")
        .ephemeral(true)
        .log_destination(log_file)
        .build()?;
    println!("   ✓ Logging to /tmp/tailscale_demo.log\n");
    drop(ts2);

    // Example 3: Discard all logs
    println!("3. Creating Tailscale with logging disabled...");
    let ts3 = Tailscale::builder()
        .hostname("demo-no-log")
        .ephemeral(true)
        .log_discard()
        .build()?;
    println!("   ✓ Logging disabled\n");
    drop(ts3);

    println!("=== Demo Complete ===");
    Ok(())
}
