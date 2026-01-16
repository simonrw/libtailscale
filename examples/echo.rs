use std::io::Read;
use tailscale2::*;

fn handle_connection(mut conn: Connection) {
    let mut buf = [0u8; 2048];
    loop {
        let i = conn.read(&mut buf).unwrap();
        if i == 0 {
            eprintln!("connection dropped");
            break;
        }

        if let Ok(value) = std::str::from_utf8(&buf[..i]) {
            println!("{}", value.trim());
        }
    }
}

fn main() {
    let ts = Tailscale::builder()
        .ephemeral(true)
        .hostname("foo")
        .build()
        .unwrap();
    ts.up().unwrap();

    let listener = ts.listener("tcp", ":1999").unwrap();
    eprintln!("listening for connections");
    std::thread::scope(|s| {
        loop {
            let conn = listener.accept().unwrap();
            if let Some(addr) = conn.remote_addr().unwrap() {
                eprintln!("got connection from {}", addr);
            }
            s.spawn(move || handle_connection(conn));
        }
    })
}
