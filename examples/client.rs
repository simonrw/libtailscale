use tailscale2::*;
use tokio::io::AsyncWriteExt;

#[tokio::main]
async fn main() {
    let ts = Tailscale::builder()
        .ephemeral(true)
        .hostname("foo")
        .build()
        .unwrap();
    ts.up().await.unwrap();

    let mut conn = ts.connect("tcp", "mm:8000").await.unwrap();
    println!("connection established");

    let text = "hello".to_string().into_bytes();
    conn.write_all(&text).await.unwrap();
}
