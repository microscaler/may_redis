# Client API Design

## API Surface

The API mirrors the `redis` crate's `Commands` trait for familiar ergonomics,
but replaces async-await with may-coroutine yields.

### 1. Command Builder

```rust
// Build a command (no execution, just encoding)
let cmd = cmd("SET").arg("key").arg("value").arg("EX").arg(60u32);
let bytes = cmd.build(); // BytesMut — the RESP wire format

// Or execute directly:
let resp: Result<String, RedisError> = client.execute(cmd("GET").arg("key"))?;
```

### 2. Client API

```rust
// Connect (creates a connection, starts the connection loop)
let client = RedisClient::connect("redis://localhost:6379").await?;

// Single command execution (blocking in coroutine sense)
let value: Result<String, RedisError> = client.get("key").await?;

// Pipeline (multiple commands, one round-trip)
let (r1, r2, r3) = client.pipeline(|p| {
    p.get("key1");
    p.get("key2");
    p.get("key3");
}).await?;

// Raw command execution
let resp: Result<i64, RedisError> = client.execute(cmd("INCR").arg("counter")).await?;
```

### 3. Commands Trait

```rust
pub trait Commands: Sized {
    fn get<K: ToRedisArgs, V: FromRedisValue>(&self, key: K) -> RedisResult<Vec<V>>;
    fn set<K: ToRedisArgs, V: ToRedisArgs>(&self, key: K, value: V) -> RedisResult<()>;
    fn set_ex<K: ToRedisArgs, V: ToRedisArgs>(&self, key: K, value: V, seconds: u32) -> RedisResult<()>;
    fn exists<K: ToRedisArgs>(&self, key: K) -> RedisResult<usize>;
    fn del<K: ToRedisArgs>(&self, key: K) -> RedisResult<usize>;
    fn incr<K: ToRedisArgs>(&self, key: K) -> RedisResult<i64>;
    fn ttl<K: ToRedisArgs>(&self, key: K) -> RedisResult<i64>;
    fn expire<K: ToRedisArgs>(&self, key: K, seconds: u32) -> RedisResult<bool>;
    fn publish<K: ToRedisArgs, M: ToRedisArgs>(&self, channel: K, message: M) -> RedisResult<usize>;
    fn keys<K: ToRedisArgs>(&self, pattern: K) -> RedisResult<Vec<String>>;
    fn dbsize(&self) -> RedisResult<usize>;
    fn flushdb(&self) -> RedisResult<()>;
    fn ping(&self) -> RedisResult<String>;
    fn auth(&self, password: &str) -> RedisResult<String>;
}
```

Each method:
1. Encodes the command into `BytesMut`
2. Pushes to the request queue
3. Co-yields waiting for response
4. Decodes and returns typed result

### 4. Pipeline API

```rust
// Pipeline pattern — send all commands, read all responses
struct Pipeline<'a> {
    client: &'a RedisClient,
    commands: Vec<BytesMut>,
}

impl<'a> Pipeline<'a> {
    fn new(client: &'a RedisClient) -> Self { ... }
    
    fn add(&mut self, cmd: BytesMut) -> &mut Self { ... }
    
    async fn execute<T: FromPipelineResponse>(&self) -> RedisResult<T> { ... }
}
```

Pipeline works by:
1. Send all commands to request queue (no waits)
2. Read responses in order from spsc channels
3. Decode into tuple of typed results

### 5. Typed Results

```rust
// Extract from response into Rust types
let value: String = response.extract()?;
let count: i64 = response.extract()?;
let items: Vec<String> = response.extract()?;
let ok: () = response.extract()?;
```

### 6. Connection Lifecycle

```mermaid
stateDiagram-v2
    [*] --> Disconnected
    Disconnected --> Connecting: RedisClient::connect(url)
    Connecting --> Connected: connection loop started
    Connected --> Disconnecting: client dropped
    Disconnecting --> Disconnected: connection loop ended
    Connected --> Connecting: reconnect on error
    
    state Connected {
        [*] --> Idle
        Idle --> Executing: client.get/set/etc
        Executing --> Idle: response received
    }
```

