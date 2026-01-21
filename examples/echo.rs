use tailscale2::*;
use tokio::io::AsyncReadExt;
use tracing::{debug, info};

async fn handle_connection(mut conn: Connection) {
    let mut buf = [0u8; 2048];
    loop {
        let i = conn.read(&mut buf).await.unwrap();
        if i == 0 {
            debug!("connection dropped");
            break;
        }

        if let Ok(value) = std::str::from_utf8(&buf[..i]) {
            println!("{}", value.trim());
        }
    }
}

#[tokio::main]
async fn main() {
    // Initialize tracing subscriber
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let ts = Tailscale::builder()
        .ephemeral(true)
        .hostname("foo")
        .build()
        .unwrap();
    ts.up().await.unwrap();

    let listener = ts.listener(NetworkType::Tcp, ":1999").await.unwrap();
    info!("listening for connections");
    loop {
        let conn = listener.accept().await.unwrap();
        if let Some(addr) = conn.remote_addr().unwrap() {
            info!("got connection from {}", addr);
        }
        // Spawn a new task to handle this connection concurrently
        tokio::spawn(async move {
            handle_connection(conn).await;
        });
    }
}
