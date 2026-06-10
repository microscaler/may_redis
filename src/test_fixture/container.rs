#![cfg(feature = "test")]
//! Bollard container creation and readiness polling.

use super::{runtime, DockerBuildError, RedisTestFixture};
use bollard::models::{ContainerCreateBody, HostConfig, PortBinding};
use bollard::query_parameters::{
    CreateContainerOptionsBuilder, InspectContainerOptions, StartContainerOptions,
};
use bollard::Docker;
use std::collections::HashMap;
use std::env;
use std::net::TcpStream;
use std::path::{Path, PathBuf};

/// A single managed Redis container.
pub struct RedisContainer {
    pub(super) id: String,
    pub(super) docker: Docker,
    pub(super) host_port: u16,
}

impl RedisContainer {
    fn wait_until_ready(&self) -> Result<(), DockerBuildError> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        let addr: std::net::SocketAddr = format!("127.0.0.1:{}", self.host_port)
            .parse()
            .expect("host_port is a valid u16");
        loop {
            if TcpStream::connect_timeout(&addr, std::time::Duration::from_millis(100))
                .is_ok()
            {
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

/// Builder for [`RedisTestFixture`].
pub struct RedisTestFixtureBuilder {
    plain_redis: bool,
    tls_redis: bool,
    tls_cert_dir: PathBuf,
}

impl RedisTestFixtureBuilder {
    pub(super) fn new() -> Self {
        let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| {
            env::current_dir()
                .expect("current dir")
                .to_str()
                .expect("utf8 path")
                .to_string()
        });
        Self {
            plain_redis: true,
            tls_redis: true,
            tls_cert_dir: PathBuf::from(manifest_dir).join("tests/tls"),
        }
    }

    #[must_use]
    pub fn with_plain_redis(mut self, enabled: bool) -> Self {
        self.plain_redis = enabled;
        self
    }

    #[must_use]
    pub fn with_tls_redis(mut self, enabled: bool) -> Self {
        self.tls_redis = enabled;
        self
    }

    #[must_use]
    pub fn tls_cert_dir(mut self, dir: PathBuf) -> Self {
        self.tls_cert_dir = dir;
        self
    }

    /// Build and start the configured Redis containers.
    ///
    /// # Errors
    ///
    /// Returns [`DockerBuildError`] if Docker is unavailable or container
    /// creation, start, or readiness checks fail.
    pub fn build(self) -> Result<RedisTestFixture, DockerBuildError> {
        runtime::block_on(async {
            let docker = Docker::connect_with_socket_defaults().map_err(|e| {
                DockerBuildError::DockerNotAvailable(format!("Failed to connect: {e}"))
            })?;

            remove_legacy_containers(&docker).await;

            let mut containers = Vec::new();
            if self.plain_redis {
                containers.push(create_plain_redis(&docker).await?);
            }
            if self.tls_redis {
                containers.push(create_tls_redis(&docker, &self.tls_cert_dir).await?);
            }

            let fixture = RedisTestFixture { containers };
            for container in &fixture.containers {
                container.wait_until_ready()?;
            }
            Ok(fixture)
        })
    }
}

/// Fixed, process-independent container names so fixture containers are
/// reused across test processes instead of leaking one pair per test run.
fn container_name(variant: &str) -> String {
    format!("may-redis-{variant}")
}

/// Remove containers leaked by the old pid-suffixed naming scheme
/// (`may-redis-plain-<pid>` / `may-redis-tls-<pid>`). Those containers were
/// held in a static fixture whose `Drop` never ran, so they accumulated.
async fn remove_legacy_containers(docker: &Docker) {
    use bollard::query_parameters::{
        ListContainersOptionsBuilder, RemoveContainerOptionsBuilder,
    };

    let list_opts = ListContainersOptionsBuilder::default().all(true).build();
    let Ok(summaries) = docker.list_containers(Some(list_opts)).await else {
        return;
    };
    for summary in summaries {
        let is_legacy = summary.names.iter().flatten().any(|n| {
            let n = n.trim_start_matches('/');
            ["may-redis-plain-", "may-redis-tls-"].iter().any(|prefix| {
                n.strip_prefix(prefix).is_some_and(|suffix| {
                    !suffix.is_empty() && suffix.bytes().all(|b| b.is_ascii_digit())
                })
            })
        });
        if is_legacy {
            if let Some(id) = &summary.id {
                let opts = RemoveContainerOptionsBuilder::default().force(true).build();
                let _ = docker.remove_container(id, Some(opts)).await;
            }
        }
    }
}

/// Reuse an already-running container with the given name, if any.
async fn reuse_running(
    docker: &Docker,
    name: &str,
    port_key: &str,
    fallback_port: u16,
) -> Option<RedisContainer> {
    let inspect = docker
        .inspect_container(name, None::<InspectContainerOptions>)
        .await
        .ok()?;
    let running = inspect
        .state
        .as_ref()
        .and_then(|state| state.running)
        .unwrap_or(false);
    if !running {
        return None;
    }
    let id = inspect.id.clone()?;
    Some(RedisContainer {
        id,
        docker: docker.clone(),
        host_port: mapped_port(&inspect, port_key, fallback_port),
    })
}

/// Force-remove a container by name, ignoring "no such container" errors.
async fn remove_by_name(docker: &Docker, name: &str) {
    use bollard::query_parameters::RemoveContainerOptionsBuilder;
    let opts = RemoveContainerOptionsBuilder::default().force(true).build();
    let _ = docker.remove_container(name, Some(opts)).await;
}

/// Get-or-create a named fixture container: reuse it when already running,
/// replace it when stopped, and tolerate create races with concurrent test
/// processes by falling back to the winner's container.
async fn ensure_container(
    docker: &Docker,
    name: &str,
    port_key: &str,
    fallback_port: u16,
    cfg: ContainerCreateBody,
) -> Result<RedisContainer, DockerBuildError> {
    if let Some(existing) = reuse_running(docker, name, port_key, fallback_port).await {
        return Ok(existing);
    }
    // A stopped leftover with the same name would make create fail.
    remove_by_name(docker, name).await;

    let create_opts = CreateContainerOptionsBuilder::default().name(name).build();
    let id = match docker.create_container(Some(create_opts), cfg).await {
        Ok(created) => created.id,
        Err(e) => {
            // 409 conflict: a concurrent test process created it first.
            if format!("{e}").contains("409") {
                if let Some(existing) =
                    reuse_running(docker, name, port_key, fallback_port).await
                {
                    return Ok(existing);
                }
            }
            return Err(DockerBuildError::ContainerCreate(format!("{e}")));
        }
    };
    docker
        .start_container(&id, None::<StartContainerOptions>)
        .await
        .map_err(|e| DockerBuildError::ContainerStart(format!("{e}")))?;
    let inspect = docker
        .inspect_container(&id, None::<InspectContainerOptions>)
        .await
        .map_err(|e| DockerBuildError::ContainerCreate(format!("inspect: {e}")))?;
    Ok(RedisContainer {
        id,
        docker: docker.clone(),
        host_port: mapped_port(&inspect, port_key, fallback_port),
    })
}

fn mapped_port(
    inspect: &bollard::models::ContainerInspectResponse,
    port_key: &str,
    fallback: u16,
) -> u16 {
    inspect
        .network_settings
        .as_ref()
        .and_then(|ns| ns.ports.as_ref())
        .and_then(|ports| ports.get(port_key))
        .and_then(|bindings| bindings.as_ref())
        .and_then(|vec| vec.first())
        .and_then(|binding| binding.host_port.as_ref())
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(fallback)
}

async fn create_plain_redis(
    docker: &Docker,
) -> Result<RedisContainer, DockerBuildError> {
    let name = container_name("plain");
    let port_key = "6379/tcp".to_string();
    let bindings = HashMap::from([(
        port_key.clone(),
        Some(vec![PortBinding {
            host_ip: Some("127.0.0.1".into()),
            host_port: Some("0".into()),
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
    ensure_container(docker, &name, &port_key, 6379, cfg).await
}

async fn create_tls_redis(
    docker: &Docker,
    cert_dir: &Path,
) -> Result<RedisContainer, DockerBuildError> {
    let name = container_name("tls");
    let cert_mount = cert_dir.to_str().ok_or_else(|| {
        DockerBuildError::ContainerCreate("TLS cert dir is not valid UTF-8".into())
    })?;
    let port_key = "6380/tcp".to_string();
    let bindings = HashMap::from([(
        port_key.clone(),
        Some(vec![PortBinding {
            host_ip: Some("127.0.0.1".into()),
            host_port: Some("0".into()),
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
            "--tls-auth-clients".to_string(),
            "no".to_string(),
        ]),
        ..Default::default()
    };
    ensure_container(docker, &name, &port_key, 6380, cfg).await
}
