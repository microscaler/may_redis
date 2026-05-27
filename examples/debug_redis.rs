// Minimal reproduction to debug the hang
use may::net::TcpStream;
use std::io::{Read, Write};

fn main() {
    println!("Test: can we connect at all?");

    match TcpStream::connect("127.0.0.1:6379") {
        Ok(mut stream) => {
            println!("TCP connected!");
            let ping = b"*1\r\n$4\r\nPING\r\n";
            stream.write_all(ping).unwrap();
            println!("SENT PING");
            let mut buf = [0u8; 1024];
            let n = stream.read(&mut buf).unwrap();
            println!("Got {} bytes: {}", n, std::str::from_utf8(&buf[..n]).unwrap_or("<binary>"));
        }
        Err(e) => println!("TCP connect error: {}", e),
    }

    println!("Done");
}
