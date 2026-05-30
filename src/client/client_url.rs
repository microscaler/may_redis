// URL parsing, TLS config building, and `connect_url` for `redis://` and
// `rediss://` schemes.
//
// This module was extracted from `client.rs` to keep that file under the
// 350-line limit. It owns the full URL → TLS config → authenticated client
// path.

use std::time::Duration;

use crate::core::RedisError;
use crate::protocol::builder::CommandBuilder;

// ---------------------------------------------------------------------------
// Connection scheme & helpers
// ---------------------------------------------------------------------------

/// Connection scheme: plain TCP or TLS.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)] // TLS support planned for future epics
pub(super) enum ConnectionScheme {
    Plain,
    Tls,
}

/// Return the default port for the given connection scheme.
const fn default_port(scheme: ConnectionScheme) -> u16 {
    match scheme {
        ConnectionScheme::Plain => 6379,
        ConnectionScheme::Tls => 6380,
    }
}

// ---------------------------------------------------------------------------
// URL decoding helper
// ---------------------------------------------------------------------------

/// URL-decode a percent-encoded string.
///
/// Only valid `%HH` sequences are decoded; all other characters pass through
/// unchanged. Invalid percent-encoding (e.g. `%GG`) returns a `Parse` error.
/// O(n) with no backtracking.
pub fn url_decode(s: &str) -> Result<String, RedisError> {
    let mut result = String::new();
    let mut chars = s.chars();

    while let Some(ch) = chars.next() {
        if ch == '%' {
            let hi = chars.next().ok_or_else(|| {
                RedisError::Parse("incomplete percent-encoding at end of string".into())
            })?;
            let lo = chars.next().ok_or_else(|| {
                RedisError::Parse("incomplete percent-encoding (missing second hex digit)".into())
            })?;

            let byte = u8::from_str_radix(&format!("{hi}{lo}"), 16).map_err(|_| {
                RedisError::Parse(format!(
                    "invalid percent-encoding %{hi}{lo} (not valid hex)"
                ))
            })?;

            result.push(byte as char);
        } else {
            result.push(ch);
        }
    }

    Ok(result)
}

// ---------------------------------------------------------------------------
// connect_url — URL-based connection with optional TLS + AUTH
// ---------------------------------------------------------------------------

/// Connect to a Redis server given a URL.
///
/// # Supported formats
///
/// * `redis://:password@host:port` — plain TCP with AUTH (Redis < 6)
/// * `redis://user:***@host:port` — plain TCP with username + password
/// * `rediss://host:port` — TLS (port defaults to 6380)
/// * `rediss://:password@host:port` — TLS + AUTH
///
/// # TLS support (rediss://)
///
/// TLS URLs use `--features tls` at build time. Query parameters:
///
/// * `timeout=N` — connection timeout in seconds (default: 5)
/// * `ca_cert=/path/to/ca.pem` — custom CA certificate path(s), comma-separated
/// * `client_cert=/path/to/client.pem` — client certificate for mTLS
/// * `client_key=/path/to/client-key.pem` — client private key for mTLS
/// * `verify_server=true|false` — disable hostname verification (default: true)
///
/// # URL encoding
///
/// Passwords and usernames are URL-decoded before use. This allows
/// passwords containing `@`, `:`, `/`, `?`, `#`, `[`, `]`, `%` to be
/// represented in URLs via percent-encoding.
///
/// # Errors
///
/// Returns [`RedisError::Parse`] if the URL has an unsupported scheme,
/// invalid port, unclosed IPv6 bracket, double prefix, or if the AUTH
/// command fails after a successful connection.
#[allow(clippy::too_many_lines)]
pub fn connect_url(url: &str) -> Result<super::client::RedisClient, RedisError> {
    // Issue #18: Reject double prefixes
    let (is_tls, after_scheme) = if let Some(rest) = url.strip_prefix("rediss://") {
        if rest.starts_with("rediss://") {
            return Err(RedisError::Parse(
                "double URL scheme prefix (rediss://rediss://)".into(),
            ));
        }
        (true, rest)
    } else if let Some(rest) = url.strip_prefix("redis://") {
        (false, rest)
    } else {
        return Err(RedisError::Parse(
            "must use 'redis://' or 'rediss://' prefix".into(),
        ));
    };

    // Split off query parameters
    let (path_part, query_params) = match after_scheme.split_once('?') {
        Some((path, query)) => (path, Some(query)),
        None => (after_scheme, None),
    };

    // Parse query parameters for TLS config
    let mut ca_cert_paths: Option<String> = None;
    let mut client_cert_path: Option<String> = None;
    let mut client_key_path: Option<String> = None;
    let mut timeout_secs: u32 = 5;
    let mut verify_server = true;

    if let Some(query) = query_params {
        for param in query.split('&') {
            if let Some((key, value)) = param.split_once('=') {
                match key {
                    "timeout" => {
                        timeout_secs = value
                            .parse()
                            .map_err(|_| RedisError::Parse("invalid timeout value".into()))?;
                    }
                    "ca_cert" => {
                        ca_cert_paths = Some(value.to_string());
                    }
                    "client_cert" => {
                        client_cert_path = Some(value.to_string());
                    }
                    "client_key" => {
                        client_key_path = Some(value.to_string());
                    }
                    "verify_server" => {
                        verify_server = value.parse::<bool>().map_err(|_| {
                            RedisError::Parse(
                                "invalid verify_server value (expected true/false)".into(),
                            )
                        })?;
                    }
                    _ => {} // Ignore unknown params
                }
            }
        }
    }

    // Parse user:password@host:port — use rfind('@') to correctly handle
    // passwords containing '@' (RFC 3986 §3.2.1).
    let (password, host_part) = path_part.rfind('@').map_or((None, path_part), |idx| {
        let password = &path_part[..idx];
        let host_part = &path_part[idx + 1..];
        if password.is_empty() {
            (None, host_part)
        } else {
            (Some(password), host_part)
        }
    });

    // URL-decode the password
    let password: Option<String> = password.map(url_decode).transpose()?;

    // Parse host:port — handle IPv6 [::1]:6379 and IPv4 127.0.0.1:6379
    let default_port = if is_tls {
        default_port(ConnectionScheme::Tls)
    } else {
        default_port(ConnectionScheme::Plain)
    };

    let (host, port) = if host_part.starts_with('[') {
        if let Some(close_bracket) = host_part.find(']') {
            let host = &host_part[1..close_bracket];
            let port_part = &host_part[close_bracket + 1..];
            let port: u16 = port_part
                .strip_prefix(':')
                .ok_or_else(|| RedisError::Parse("missing port for IPv6 address".into()))?
                .parse()
                .map_err(|e| RedisError::Parse(format!("invalid port: {e}")))?;
            (host, port)
        } else {
            return Err(RedisError::Parse("unclosed '[' in IPv6 address".into()));
        }
    } else {
        host_part
            .rfind(':')
            .map(|colon_idx| {
                let host = &host_part[..colon_idx];
                let port_str = &host_part[colon_idx + 1..];
                let port: u16 = port_str
                    .parse()
                    .map_err(|e| RedisError::Parse(format!("invalid port: {e}")))?;
                Ok::<_, RedisError>((host, port))
            })
            .transpose()?
            .map_or_else(|| (host_part, default_port), |(h, p)| (h, p))
    };

    if is_tls {
        // Build TLS config
        #[cfg(not(feature = "tls"))]
        {
            let _ = (
                ca_cert_paths,
                client_cert_path,
                client_key_path,
                verify_server,
            );
            return Err(RedisError::Parse(
                "TLS support not enabled — rebuild with `--features tls`".into(),
            ));
        }

        #[cfg(feature = "tls")]
        {
            // Build root certificates
            let root_certs =
                ca_cert_paths.map_or(crate::tls::config::RustlsRootCerts::WebPkiRoots, |paths| {
                    crate::tls::config::RustlsRootCerts::Pem(
                        paths
                            .split(',')
                            .map(|p| std::path::PathBuf::from(p.trim()))
                            .collect(),
                    )
                });

            // Build client certs if provided
            let client_certs = match (client_cert_path, client_key_path) {
                (Some(cert_path), Some(key_path)) => {
                    let cert_data = std::fs::read(&cert_path).map_err(|e| {
                        RedisError::Parse(format!("failed to read client cert {cert_path}: {e}"))
                    })?;
                    let key_data = std::fs::read(&key_path).map_err(|e| {
                        RedisError::Parse(format!("failed to read client key {key_path}: {e}"))
                    })?;
                    Some(
                        crate::tls::config::ClientCerts::from_pem(&cert_data, &key_data).map_err(
                            |e| RedisError::Parse(format!("failed to parse client certs: {e}")),
                        )?,
                    )
                }
                _ => None,
            };

            let tls_config = crate::tls::TlsConfig {
                root_certificates: root_certs,
                client_certs,
                server_name: host.to_string(),
                min_version: crate::tls::config::TlsVersion::Tls12,
                max_version: crate::tls::config::TlsVersion::Tls13,
                verify_server,
            };

            let client =
                super::client::RedisClient::connect_tls(host, port, &tls_config, timeout_secs)
                    .map_err(|e| RedisError::Parse(format!("TLS connection failed: {e}")))?;

            // Send AUTH if password was provided in URL
            if let Some(pass) = password {
                let auth_cmd = CommandBuilder::new("AUTH").arg(pass);
                client
                    .execute::<String>(auth_cmd)
                    .map_err(|e| RedisError::Parse(format!("AUTH failed: {e}")))?;
            }

            Ok(client)
        }
    } else {
        // Plain TCP connection
        let client = super::client::RedisClient::connect_with_timeout(
            host,
            port,
            Duration::from_secs(u64::from(timeout_secs)),
        )
        .map_err(|e| RedisError::Parse(format!("connection failed: {e}")))?;

        // Send AUTH if password was provided in URL
        if let Some(pass) = password {
            let auth_cmd = CommandBuilder::new("AUTH").arg(pass);
            client
                .execute::<String>(auth_cmd)
                .map_err(|e| RedisError::Parse(format!("AUTH failed: {e}")))?;
        }

        Ok(client)
    }
}
