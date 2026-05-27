// Minimal debug: connect to Redis, send PING, print response
use may_redis::RedisClient;
use std::time::Duration;

fn main() {
    println!("Connecting to 127.0.0.1:6379...");
    match RedisClient::connect("127.0.0.1", 6379) {
        Ok(client) => {
            println!("Connected! id={}", client.id());
            println!("Sending PING...");
            match client.ping() {
                Ok(response) => println!("PING response: {}", response),
                Err(e) => eprintln!("PING error: {}", e),
            }
        }
        Err(e) => eprintln!("Connect error: {}", e),
    }
    println!("Done.");
}
