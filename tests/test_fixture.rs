// Test fixture — manages Redis/Redis-TLS containers via bollard.
//
// Pattern from BRRTRouter/tests/docker_integration_tests.rs:
//   - ContainerCreateBody for container config
//   - CreateContainerOptionsBuilder for creation options
//   - RemoveContainerOptionsBuilder for cleanup options
//   - futures::executor::block_on for sync API calls
//   - Bind port "0" then read back from inspect_container
//   - RAII Drop for automatic cleanup (mirrors DockerTestContainer)
//
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use bollard::models::{ContainerCreateBody, HostConfig, PortBinding};
use bollard::query_parameters::{
    CreateContainerOptionsBuilder, InspectContainerOptions,
    RemoveContainerOptionsBuilder, StartContainerOptions,
};
use bollard::Docker;
use futures::executor::block_on;
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::sync::OnceLock;

/// A managed Redis container.
pub struct RedisContainer {
    id: String,
    _docker: Docker,
    /// The host port mapped to the container's Redis port.
    host_port: u16,
}

/// A Redis test fixture that manages one or more containers.
///
/// Mirrors BRRTRouter's DockerTestContainer pattern:
/// automatic RAII cleanup on drop.
pub struct RedisTestFixture {
    containers: Vec<RedisContainer>,
}

impl RedisTestFixture {
    /// Create a builder for constructing a test fixture.
    pub fn builder() -> RedisTestFixtureBuilder {
        RedisTestFixtureBuilder::new()
    }

    /// Get the host port for the container at index `i`.
    pub fn host(&self, i: usize) -> u16 {
        self.containers[i].host_port
    }

    /// Get the number of containers in this fixture.
    pub fn len(&self) -> usize {
        self.containers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.containers.is_empty()
    }
}

impl Drop for RedisTestFixture {
    fn drop(&mut self) {
        // Mirror BRRTRouter's DockerTestContainer::drop pattern:
        // Spawn a thread with a tokio runtime, block_on the async cleanup.
        for container in self.containers.drain(..) {
            let id = container.id.clone();
            let docker = container._docker.clone();
            std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();
                rt.block_on(async {
                    let opts = RemoveContainerOptionsBuilder::default()
                        .force(true)
                        .build();
                    let _ = docker.remove_container(&id, Some(opts)).await;
                });
            });
        }
    }
}

/// Error type for Docker build failures.
#[derive(Debug)]
pub enum DockerBuildError {
    /// Docker daemon is not running
    DockerNotAvailable(String),
    /// Container creation failed
    ContainerCreate(String),
    /// Container failed to start
    ContainerStart(String),
    /// Container did not become ready in time
    ContainerNotReady(String),
}

impl std::fmt::Display for DockerBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DockerNotAvailable(msg) => write!(f, "Docker not available: {msg}"),
            Self::ContainerCreate(msg) => write!(f, "Container creation failed: {msg}"),
            Self::ContainerStart(msg) => write!(f, "Container start failed: {msg}"),
            Self::ContainerNotReady(msg) => write!(f, "Container not ready: {msg}"),
        }
    }
}

impl std::error::Error for DockerBuildError {}

/// Cached Docker availability check — one connection attempt per test process.
static DOCKER_AVAILABLE: OnceLock<bool> = OnceLock::new();

/// Check if Docker is available and running.
/// Mirrors BRRTRouter's is_docker_available() pattern.
pub fn is_docker_available() -> bool {
    *DOCKER_AVAILABLE.get_or_init(|| {
        Docker::connect_with_socket_defaults().is_ok()
    })
}

/// Builder for constructing a RedisTestFixture.
pub struct RedisTestFixtureBuilder {
    plain_redis: bool,
    tls_redis: bool,
    tls_cert_dir: PathBuf,
}

impl RedisTestFixtureBuilder {
    fn new() -> Self {
        let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| {
            env::current_dir().unwrap().to_str().unwrap().to_string()
        });
        Self {
            plain_redis: true,
            tls_redis: true,
            tls_cert_dir: PathBuf::from(manifest_dir).join("tests/tls"),
        }
    }

    /// Include a plain Redis container (default: yes).
    pub fn with_plain_redis(mut self, enabled: bool) -> Self {
        self.plain_redis = enabled;
        self
    }

    /// Include a Redis-TLS container (default: yes).
    pub fn with_tls_redis(mut self, enabled: bool) -> Self {
        self.tls_redis = enabled;
        self
    }

    /// Custom path to TLS certificates.
    pub fn tls_cert_dir(mut self, dir: PathBuf) -> Self {
        self.tls_cert_dir = dir;
        self
    }

    /// Build the fixture — creates containers and waits for them to be ready.
    pub fn build(self) -> Result<RedisTestFixture, DockerBuildError> {
        // Connect to Docker (mirrors BRRTRouter's Docker::connect_with_local_defaults)
        let docker = Docker::connect_with_socket_defaults()
            .map_err(|e| DockerBuildError::DockerNotAvailable(format!("Failed to connect to Docker: {e}")))?;

        let mut containers = Vec::new();

        if self.plain_redis {
            let container = self.create_plain_redis(&docker)?;
            containers.push(container);
        }

        if self.tls_redis {
            let container = self.create_tls_redis(&docker)?;
            containers.push(container);
        }

        let fixture = RedisTestFixture { containers };

        // Wait for all containers to be ready.
        for container in &fixture.containers {
            container.wait_until_ready()?;
        }

        Ok(fixture)
    }

    fn create_plain_redis(&self, docker: &Docker) -> Result<RedisContainer, DockerBuildError> {
        // Mirror BRRTRouter pattern: build port bindings, host_config, ContainerCreateBody
        let name = self.container_name("plain");
        let port_key = "6379/tcp".to_string();
        let bindings = HashMap::from([(
            port_key.clone(),
            Some(vec![PortBinding {
                host_ip: Some("127.0.0.1".into()),
                host_port: Some("0".into()),  // bind to random port
            }]),
        )]);

        let host_config = HostConfig {
            port_bindings: Some(bindings),
            ..Default::default()
        };

        let cfg = ContainerCreateBody {
            image: Some("redis:7-alpine".to_string()),
            host_config: Some(host_config),
            cmd: Some(vec![
                "redis-server".to_string(),
                "--loglevel".to_string(),
                "warning".to_string(),
            ]),
            ..Default::default()
        };

        let create_opts = CreateContainerOptionsBuilder::default()
            .name(&name)
            .build();

        let created = block_on(docker.create_container(Some(create_opts), cfg))
            .map_err(|e| DockerBuildError::ContainerCreate(format!("{e}")))?;

        block_on(docker.start_container(&created.id, None::<StartContainerOptions>))
            .map_err(|e| DockerBuildError::ContainerStart(format!("{e}")))?;

        // Read back the mapped port from inspect (mirrors BRRTRouter's inspect pattern)
        let inspect = block_on(docker.inspect_container(
            &created.id,
            None::<InspectContainerOptions>,
        ))
        .map_err(|e| DockerBuildError::ContainerCreate(format!("Failed to inspect: {e}")))?;

        let mapped_port = inspect
            .network_settings
            .as_ref()
            .and_then(|ns| ns.ports.as_ref())
            .and_then(|ports| ports.get(&port_key))
            .and_then(|bindings| bindings.as_ref())
            .and_then(|vec| vec.first())
            .and_then(|binding| binding.host_port.as_ref())
            .and_then(|s| s.parse::<u16>().ok())
            .unwrap_or(6379);

        Ok(RedisContainer {
            id: created.id,
            _docker: docker.clone(),
            host_port: mapped_port,
        })
    }

    fn create_tls_redis(&self, docker: &Docker) -> Result<RedisContainer, DockerBuildError> {
        let name = self.container_name("tls");
        let cert_mount = self.tls_cert_dir.to_str().unwrap().to_string();
        let port_key = "6380/tcp".to_string();
        let bindings = HashMap::from([(
            port_key.clone(),
            Some(vec![PortBinding {
                host_ip: Some("127.0.0.1".into()),
                host_port: Some("0".into()),  // bind to random port
            }]),
        )]);

        let host_config = HostConfig {
            port_bindings: Some(bindings),
            binds: Some(vec![format!("{cert_mount}:/tls:ro")]),
            ..Default::default()
        };

        let cfg = ContainerCreateBody {
            image: Some("redis:7-alpine".to_string()),
            host_config: Some(host_config),
            cmd: Some(vec![
                "redis-server".to_string(),
                "--tls-port".to_string(),
                "6380".to_string(),
                "--port".to_string(),
                "0".to_string(),
                "--tls-cert-file".to_string(),
                "/tls/server.crt".to_string(),
                "--tls-key-file".to_string(),
                "/tls/server.key".to_string(),
                "--tls-ca-cert-file".to_string(),
                "/tls/ca.crt".to_string(),
            ]),
            ..Default::default()
        };

        let create_opts = CreateContainerOptionsBuilder::default()
            .name(&name)
            .build();

        let created = block_on(docker.create_container(Some(create_opts), cfg))
            .map_err(|e| DockerBuildError::ContainerCreate(format!("{e}")))?;

        block_on(docker.start_container(&created.id, None::<StartContainerOptions>))
            .map_err(|e| DockerBuildError::ContainerStart(format!("{e}")))?;

        // Read back the mapped port from inspect (mirrors BRRTRouter)
        let inspect = block_on(docker.inspect_container(
            &created.id,
            None::<InspectContainerOptions>,
        ))
        .map_err(|e| DockerBuildError::ContainerCreate(format!("Failed to inspect: {e}")))?;

        let mapped_port = inspect
            .network_settings
            .as_ref()
            .and_then(|ns| ns.ports.as_ref())
            .and_then(|ports| ports.get(&port_key))
            .and_then(|bindings| bindings.as_ref())
            .and_then(|vec| vec.first())
            .and_then(|binding| binding.host_port.as_ref())
            .and_then(|s| s.parse::<u16>().ok())
            .unwrap_or(6380);

        Ok(RedisContainer {
            id: created.id,
            _docker: docker.clone(),
            host_port: mapped_port,
        })
    }

    fn container_name(&self, variant: &str) -> String {
        format!("may-redis-{variant}-{}", std::process::id())
    }
}

impl RedisContainer {
    /// Wait until the container is accepting connections.
    pub fn wait_until_ready(&self) -> Result<(), DockerBuildError> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            let addr: std::net::SocketAddr =
                format!("127.0.0.1:{}", self.host_port).parse().unwrap();
            if std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_millis(100)).is_ok() {
                return Ok(());
            }
            if std::time::Instant::now() > deadline {
                return Err(DockerBuildError::ContainerNotReady(
                    "Redis container did not become ready in time".into(),
                ));
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
}
