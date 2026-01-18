use tailscale2::*;
use tokio::io::AsyncReadExt;

async fn handle_connection(mut conn: Connection) {
    let mut buf = [0u8; 2048];
    loop {
        let i = conn.read(&mut buf).await.unwrap();
        if i == 0 {
            eprintln!("connection dropped");
            break;
        }

        if let Ok(value) = std::str::from_utf8(&buf[..i]) {
            println!("{}", value.trim());
        }
    }
}

#[tokio::main]
async fn main() {
    let ts = Tailscale::builder()
        .ephemeral(true)
        .hostname("foo")
        .build()
        .unwrap();
    ts.up().await.unwrap();

    let listener = ts.listener("tcp", ":1999").await.unwrap();
    eprintln!("listening for connections");
    loop {
        let conn = listener.accept().await.unwrap();
        if let Some(addr) = conn.remote_addr().unwrap() {
            eprintln!("got connection from {}", addr);
        }
        // Spawn a new task to handle this connection concurrently
        tokio::spawn(async move {
            handle_connection(conn).await;
        });
    }
}
