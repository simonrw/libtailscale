use tailscale2::*;
use tokio::io::AsyncWriteExt;
use tracing::info;

#[tokio::main]
async fn main() {
    // Initialize tracing subscriber
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        )
        .init();

    let ts = Tailscale::builder()
        .ephemeral(true)
        .hostname("foo")
        .build()
        .unwrap();
    ts.up().await.unwrap();

    let mut conn = ts.connect(NetworkType::Tcp, "mm:8000").await.unwrap();
    info!("connection established");

    let text = "hello".to_string().into_bytes();
    conn.write_all(&text).await.unwrap();
}
