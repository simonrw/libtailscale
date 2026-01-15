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
    let ts = Tailscale::builder().ephemeral(true).build().unwrap();
    ts.up().unwrap();

    let listener = ts.listener("tcp", ":1999").unwrap();

    eprintln!("listening for connections");

    loop {
        let conn = listener.accept().unwrap();
        eprintln!("got connection");
        std::thread::spawn(move || handle_connection(conn));
    }
}
