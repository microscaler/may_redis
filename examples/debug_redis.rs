// Minimal reproduction to debug the hang
fn main() {
    println!("Test 1: can we connect at all?");
    
    // First, let's try with a simple may::run approach
    may::run(|| {
        may::go!({
            println!("Inside go! block");
            // Try TCP connect
            use may::net::TcpStream;
            match TcpStream::connect("127.0.0.1:6379") {
                Ok(mut stream) => {
                    println!("TCP connected!");
                    // Send PING
                    let ping = b"*1\r\n$4\r\nPING\r\n";
                    stream.write_all(ping).unwrap();
                    println!("SENT PING");
                    // Read response
                    let mut buf = [0u8; 1024];
                    let n = stream.read(&mut buf).unwrap();
                    println!("Got {} bytes: {}", n, std::str::from_utf8(&buf[..n]).unwrap_or("<binary>"));
                }
                Err(e) => println!("TCP connect error: {}", e),
            }
            println!("go! block done");
        });
    });
    
    println!("After may::run");
}
