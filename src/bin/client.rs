use std::io::Write;
use tailscale2::*;

fn main() {
    let ts = Tailscale::builder()
        .ephemeral(true)
        .hostname("foo")
        .build()
        .unwrap();
    ts.up().unwrap();

    let mut conn = ts.connect("tcp", "mm:8000").unwrap();
    println!("connection established");

    let text = "hello".to_string().into_bytes();
    conn.write_all(&text).unwrap();
}
