# Analysis: BRRTRouter Test Containers with Bollard

## 1. BRRTRouter's Bollard Usage

BRRTRouter uses bollard 0.20 with features `["ssl", "chrono"]` for Docker integration testing. The primary file is `tests/docker_integration_tests.rs`.

### 1.1. Docker Client Setup

```rust
use bollard::Docker;
let docker = Docker::connect_with_local_defaults().expect("docker client");
```

Uses `connect_with_local_defaults()` — connects to Docker via local socket.

### 1.2. Container Creation Pattern

```rust
// 1. Build port bindings
let port_key = "8080/tcp".to_string();
let bindings = std::collections::HashMap::from([(
    port_key.clone(),
    Some(vec![PortBinding {
        host_ip: Some("127.0.0.1".into()),
        host_port: Some("0".into()),  // bind to random port
    }]),
)]);

// 2. Build HostConfig
let host_config = HostConfig {
    port_bindings: Some(bindings),
    ..Default::default()
};

// 3. Build container config (uses ContainerCreateBody in 0.20)
let cfg = ContainerCreateBody {
    image: Some("brrtrouter-petstore:e2e".to_string()),
    host_config: Some(host_config),
    ..Default::default()
};

// 4. Build options with builder pattern
let create_opts = CreateContainerOptionsBuilder::default()
    .name("brrtrouter-e2e")
    .build();

// 5. Create container (async, block_on)
let created = block_on(docker.create_container(Some(create_opts), cfg)).unwrap();
```

### 1.3. Container Start

```rust
block_on(docker.start_container(container.id(), None::<StartContainerOptions>)).unwrap();
```

### 1.4. Port Discovery

After starting, the container's mapped port is read back via `inspect_container`:

```rust
let inspect = block_on(docker.inspect_container(
    container.id(),
    None::<bollard::query_parameters::InspectContainerOptions>,
))
.unwrap();

let mapped_port = inspect
    .network_settings
    .and_then(|ns| ns.ports)
    .and_then(|mut p| p.remove(&port_key).flatten())
    .and_then(|mut v| v.pop())
    .and_then(|b| b.host_port)
    .unwrap()
    .parse::<u16>()
    .unwrap();
```

### 1.5. RAII Cleanup via Drop

```rust
struct DockerTestContainer {
    docker: Docker,
    container_id: String,
}

impl Drop for DockerTestContainer {
    fn drop(&mut self) {
        let opts = RemoveContainerOptionsBuilder::default().force(true).build();
        let _ = block_on(self.docker.remove_container(&self.container_id, Some(opts)));
    }
}
```

The container is automatically removed when the struct goes out of scope, even on panic.

### 1.6. Docker Availability Check

```rust
fn is_docker_available() -> bool {
    Command::new("docker")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}
```

Simple check — verifies Docker binary is present.

### 1.7. Test Skip Pattern

```rust
#[test]
fn test_petstore_container_health() {
    if std::env::var("E2E_DOCKER").is_err() {
        println!("Skipping: set E2E_DOCKER=1 to enable Docker e2e test");
        return;
    }
    if !is_docker_available() {
        println!("Skipping test: Docker not available");
        return;
    }
    // ... test code
}
```

Tests are guarded by environment variable + Docker availability check. Tests skip early if Docker is not available.

### 1.8. Image Building (for self-hosted images)

BRRTRouter also uses bollard to build Docker images from a Dockerfile:

```rust
// Build tar archive of project files
let mut archive = Vec::new();
let mut builder = TarBuilder::new(&mut archive);
// ... add files ...
builder.finish().unwrap();

// Build via Docker API
let build_opts = BuildImageOptionsBuilder::default()
    .dockerfile("dockerfiles/Dockerfile")
    .t("brrtrouter-petstore:e2e")
    .rm(true)
    .nocache(true)
    .build();
let mut stream = docker.build_image(build_opts, None, Some(body_full(Bytes::from(archive))));
while let Some(_chunk) = block_on(stream.try_next()).unwrap_or(None) {}
```

For may-redis, this is NOT needed — we'll use official `redis:7-alpine` images.

---

## 2. Key API Differences: bollard 0.20 vs 0.18

may_redis uses bollard 0.18. Key differences:

| Feature | bollard 0.20 | bollard 0.18 |
|---------|-------------|--------------|
| Container config struct | `ContainerCreateBody` | `bollard::container::Config<Z>` |
| `host_config` field | `ContainerCreateBody::host_config` | `Config::host_config` |
| Query params | `bollard::query_parameters::*` | `bollard::container::*` |
| Create options | `CreateContainerOptionsBuilder` | `bollard::container::CreateContainerOptions` |
| Start options | `StartContainerOptions` | `bollard::container::StartContainerOptions` |
| Models | `bollard::models::*` | Same |

The `ContainerCreateBody` struct was introduced in 0.19. In 0.18, use `bollard::container::Config<String>`.

---

## 3. Implementation Plan for may-redis

### 3.1. Add Dependencies

```toml
[dev-dependencies]
futures = "0.3"  # for futures::executor::block_on
```

Required because `create_container`, `start_container`, `inspect_container` are async methods on `Docker`.

### 3.2. File Structure

Add `tests/test_fixture.rs` with:

1. **`RedisContainer`** — represents a single Docker container with its ID, Docker client, and mapped host port.
2. **`RedisTestFixture`** — owns a Vec of `RedisContainer`s, provides RAII cleanup on Drop.
3. **`RedisTestFixtureBuilder`** — builder for constructing fixtures with plain Redis and/or TLS Redis.
4. **`DockerBuildError`** — error enum for Docker operations.
5. **`is_docker_available()`** — cached Docker availability check via `OnceLock`.

### 3.3. API Design

```rust
// Builder usage
let fixture = RedisTestFixture::builder()
    .with_plain_redis()        // default: true
    .with_tls_redis()          // default: true
    .tls_cert_dir(PathBuf::from("tests/tls"))
    .build()?;

// Access ports
let plain_port = fixture.host(0);   // Redis plain, random host port
let tls_port = fixture.host(1);     // Redis TLS, random host port

// Automatic cleanup when fixture drops
drop(fixture);
```

### 3.4. Container Setup

**Plain Redis** (`redis:7-alpine`):
- Command: `redis-server --loglevel warning`
- Port: 6379/tcp → host random
- No TLS, no volume mounts

**TLS Redis** (`redis:7-alpine`):
- Command: `redis-server --tls-port 6380 --port 0 --tls-cert-file /tls/server.crt --tls-key-file /tls/server.key --tls-ca-cert-file /tls/ca.crt`
- Port: 6380/tcp → host random
- Volume mount: `tests/tls/` → `/tls:ro`

### 3.5. Port Binding Strategy

Follow BRRTRouter's pattern:
1. Bind `"0"` for `host_port` in `PortBinding` — Docker assigns a random available port.
2. After container starts, call `inspect_container` to read back the actual mapped port.
3. Store the mapped port in `RedisContainer.host_port`.

Alternative: pre-bind a `TcpListener` on the host and read `local_addr().port()` — more predictable but holds the port until container creation (not ideal).

### 3.6. Docker Readiness Wait

After creating and starting the container, poll `TcpStream::connect_timeout("127.0.0.1:{port}", 100ms)` in a loop until connection succeeds (timeout 10s). This ensures Redis is accepting connections before tests run.

### 3.7. RAII Cleanup

Implement `Drop` for `RedisTestFixture`:
1. Drain containers from the Vec.
2. For each container, spawn a `std::thread::spawn` that creates a tokio runtime.
3. Inside the runtime, call `docker.remove_container(&id, Some(RemoveContainerOptions { force: true }))` via `.await`.
4. This ensures cleanup runs even if the test panics.

### 3.8. Test Skip Logic

```rust
static DOCKER_AVAILABLE: OnceLock<bool> = OnceLock::new();

pub fn is_docker_available() -> bool {
    *DOCKER_AVAILABLE.get_or_init(|| {
        Docker::connect_with_socket_defaults().is_ok()
    })
}
```

Tests check `is_docker_available()` and skip if false.

### 3.9. Integration Points

Replace `shared_client()` in integration tests with a `RedisTestFixture` that:
1. Spawns containers at test startup.
2. Provides host ports to `RedisClient::connect("127.0.0.1", port)`.
3. Auto-cleans containers on fixture drop.

### 3.10. Container Naming

Name containers `may-redis-{variant}-{pid}` for uniqueness per test process run.

---

## 4. Verification Checklist

- [ ] `cargo check --tests` passes with new fixture code
- [ ] `cargo test --lib` (unit tests only) passes without Docker
- [ ] `E2E_DOCKER=1 cargo test` passes with Docker running
- [ ] `cargo test` (without `E2E_DOCKER`) skips container tests gracefully
- [ ] `cargo clippy --workspace --all-targets` passes
- [ ] Containers are cleaned up after tests (verify with `docker ps`)
