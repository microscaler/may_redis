// Minimal reproduction to debug the hang
use may::net::TcpStream;
use std::io::{Read, Write};

fn main() {
    println!("Test: can we connect at all?");

    match TcpStream::connect("127.0.0.1:6379") {
        Ok(mut stream) => {
            println!("TCP connected!");
            let ping = b"*1\r\n$4\r\nPING\r\n";
            if let Err(e) = stream.write_all(ping) {
                println!("Write error: {e}");
                return;
            }
            println!("SENT PING");
            let mut buf = [0u8; 1024];
            match stream.read(&mut buf) {
                Ok(n) => {
                    println!(
                        "Got {n} bytes: {}",
                        std::str::from_utf8(&buf[..n]).unwrap_or("<binary>")
                    );
                }
                Err(e) => println!("Read error: {e}"),
            }
        }
        Err(e) => println!("TCP connect error: {e}"),
    }

    println!("Done");
}
