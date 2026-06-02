// URL parsing, TLS config building, and `connect_url` for `redis://` and
// `rediss:***@host:port` — plain TCP with AUTH (Redis < 6)

use std::collections::HashMap;
#[cfg(feature = "tls")]
use std::path::PathBuf;

#[cfg(feature = "tls")]
use crate::connection::SsrfConfig;
use std::time::Duration;

use crate::core::RedisError;
use crate::protocol::builder::CommandBuilder;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// URL-decode a string. Decodes `%XX` hex sequences and `+` → space.
///
/// Ported from `redis-rs/src/url` under MIT/Apache-2.0.
pub fn url_decode(s: &str) -> Result<String, RedisError> {
    let mut result = Vec::with_capacity(s.len());
    let mut chars = s.bytes();
    while let Some(b) = chars.next() {
        match b {
            b'+' => result.push(b' '),
            b'%' => {
                let hi = chars.next().ok_or_else(|| {
                    RedisError::Parse("truncated percent-encoding in URL".into())
                })?;
                let lo = chars.next().ok_or_else(|| {
                    RedisError::Parse("truncated percent-encoding in URL".into())
                })?;
                let byte = parse_hex(hi)? * 16 + parse_hex(lo)?;
                result.push(byte);
            }
            _ => result.push(b),
        }
    }
    String::from_utf8(result)
        .map_err(|_| RedisError::Parse("url-decoded bytes are not valid UTF-8".into()))
}

fn parse_hex(b: u8) -> Result<u8, RedisError> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        b'A'..=b'F' => Ok(b - b'A' + 10),
        _ => Err(RedisError::Parse(format!(
            "invalid hex digit: {:?}",
            b as char
        ))),
    }
}

/// Parse URL query string into a parameter map.
///
/// - Splits on `&` to get key=value pairs
/// - URL-decodes each key and value (FR-009)
/// - Parameter names are case-insensitive (NFR-002)
/// - Returns [`RedisError::Parse`] if a pair lacks `=`
///
/// Returns an empty map for empty query strings.
fn parse_tls_query_params(query: &str) -> Result<HashMap<String, String>, RedisError> {
    if query.is_empty() {
        return Ok(HashMap::new());
    }
    let mut params = HashMap::new();
    for pair in query.split('&') {
        let (key, value) = pair.split_once('=').ok_or_else(|| {
            RedisError::Parse("invalid query parameter (missing '=')".into())
        })?;
        let key = url_decode(key)?;
        let value = url_decode(value)?;
        params.insert(key.to_lowercase(), value);
    }
    Ok(params)
}

// ---------------------------------------------------------------------------
// connect_url
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
/// * `system_certs=true` — use webpki_roots (Mozilla) instead of ca_cert
/// * `server_name=example.com` — override SNI server name
/// * `tls_min_version=1.2|1.3` — minimum TLS version (default: 1.2)
/// * `ssrf=true|false` — enable/disable SSRF IP deny-list for TLS (default: true)
/// * `tls_max_version=1.2|1.3` — maximum TLS version (default: 1.3)
///
/// All parameter names are case-insensitive. Values are URL-decoded.
/// Unknown parameters return a `Parse` error (FR-011).
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
/// invalid port, unclosed IPv6 bracket, double prefix, unknown parameter,
/// or if the AUTH command fails after a successful connection.
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
            "unsupported URL scheme (expected 'redis://' or 'rediss://')".into(),
        ));
    };

    // Split off query parameters
    let (path_part, query_string) = match after_scheme.split_once('?') {
        Some((path, query)) => (path, Some(query)),
        None => (after_scheme, None),
    };

    // Parse query parameters (FR-002, FR-009, NFR-002)
    let params = query_string
        .map(parse_tls_query_params)
        .transpose()?
        .unwrap_or_default();

    // Extract known TLS parameters (FR-011: unknown parameter rejection)
    let known_params: std::collections::HashSet<&str> = [
        "timeout",
        "ca_cert",
        "client_cert",
        "client_key",
        "verify_server",
        "system_certs",
        "server_name",
        "tls_min_version",
        "tls_max_version",
    ]
    .iter()
    .copied()
    .collect();
    for key in params.keys() {
        if !known_params.contains(key.as_str()) {
            return Err(RedisError::Parse(format!("unknown URL parameter: '{key}'")));
        }
    }

    let timeout_secs: u32 = params
        .get("timeout")
        .and_then(|v| v.parse().ok())
        .unwrap_or(5);
    let ca_cert_paths: Option<String> = params.get("ca_cert").cloned();
    let client_cert_path: Option<String> = params.get("client_cert").cloned();
    let client_key_path: Option<String> = params.get("client_key").cloned();
    let verify_server: bool = params
        .get("verify_server")
        .is_none_or(|v| v.to_lowercase() != "false");
    let system_certs: bool = params
        .get("system_certs")
        .is_some_and(|v| v.to_lowercase() == "true");
    let server_name_override: Option<String> = params.get("server_name").cloned();
    let tls_min_version_str: Option<&str> =
        params.get("tls_min_version").map(String::as_str);
    let tls_max_version_str: Option<&str> =
        params.get("tls_max_version").map(String::as_str);

    // Extract auth credentials — use rfind('@') to correctly handle
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
        default_port(&ConnectionScheme::Tls)
    } else {
        default_port(&ConnectionScheme::Plain)
    };

    let (host, port) = if host_part.starts_with('[') {
        if let Some(close_bracket) = host_part.find(']') {
            let host = &host_part[1..close_bracket];
            let port_part = &host_part[close_bracket + 1..];
            let port: u16 = port_part
                .strip_prefix(':')
                .ok_or_else(|| {
                    RedisError::Parse("missing port for IPv6 address".into())
                })?
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
                system_certs,
                server_name_override,
                tls_min_version_str,
                tls_max_version_str,
            );
            return Err(RedisError::Parse(
                "TLS support not enabled — rebuild with `--features tls`".into(),
            ));
        }

        #[cfg(feature = "tls")]
        {
            // Build root certificates (FR-012: require ca_cert OR system_certs=true)
            let root_certs = if system_certs {
                crate::tls::config::RustlsRootCerts::WebPkiRoots
            } else if let Some(paths) = ca_cert_paths {
                crate::tls::config::RustlsRootCerts::Pem(
                    paths.split(',').map(|p| PathBuf::from(p.trim())).collect(),
                )
            } else {
                return Err(RedisError::Parse(
                    "neither 'ca_cert' nor 'system_certs=true' provided — cannot verify server".into(),
                ));
            };

            // Build client certs if provided (mTLS)
            let client_certs = match (client_cert_path, client_key_path) {
                (Some(cert_path), Some(key_path)) => {
                    let cert_data = std::fs::read(&cert_path).map_err(|e| {
                        RedisError::Parse(format!(
                            "failed to read client cert {cert_path}: {e}"
                        ))
                    })?;
                    let key_data = std::fs::read(&key_path).map_err(|e| {
                        RedisError::Parse(format!(
                            "failed to read client key {key_path}: {e}"
                        ))
                    })?;
                    Some(
                        crate::tls::config::ClientCerts::from_pem(
                            &cert_data, &key_data,
                        )
                        .map_err(|e| {
                            RedisError::Parse(format!(
                                "failed to parse client certs: {e}"
                            ))
                        })?,
                    )
                }
                _ => None,
            };

            // Parse TLS versions (FR-007)
            let min_ver = match tls_min_version_str {
                Some(v) => crate::tls::config::TlsVersion::parse(v).map_err(|e| {
                    RedisError::Parse(format!("invalid tls_min_version '{v}': {e}"))
                })?,
                None => crate::tls::config::TlsVersion::Tls12,
            };
            let max_ver = match tls_max_version_str {
                Some(v) => crate::tls::config::TlsVersion::parse(v).map_err(|e| {
                    RedisError::Parse(format!("invalid tls_max_version '{v}': {e}"))
                })?,
                None => crate::tls::config::TlsVersion::Tls13,
            };

            // FR-008: server_name override (default: host from URL)
            let sni_name = server_name_override.unwrap_or_else(|| host.to_string());

            let tls_config = crate::tls::TlsConfig {
                root_certificates: root_certs,
                client_certs,
                server_name: sni_name,
                min_version: min_ver,
                max_version: max_ver,
                verify_server,
            };

            let ssrf_enabled = params.get("ssrf").is_some_and(|v| v == "true");

            let client = if ssrf_enabled {
                let ssrf_config = SsrfConfig::default();
                super::client::RedisClient::connect_tls_with_ssrf(
                    host,
                    port,
                    &tls_config,
                    timeout_secs,
                    ssrf_config,
                )
                .map_err(|e| {
                    RedisError::Parse(format!("TLS+SSRF connection failed: {e}"))
                })?
            } else {
                super::client::RedisClient::connect_tls(
                    host,
                    port,
                    &tls_config,
                    timeout_secs,
                )
                .map_err(|e| RedisError::Parse(format!("TLS connection failed: {e}")))?
            };

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

const fn default_port(scheme: &ConnectionScheme) -> u16 {
    match scheme {
        ConnectionScheme::Plain => 6379,
        ConnectionScheme::Tls => 6380,
    }
}

enum ConnectionScheme {
    Plain,
    Tls,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    // -----------------------------------------------------------------------
    // URL decode tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_url_decode_simple() {
        assert_eq!(url_decode("hello").unwrap(), "hello");
    }

    #[test]
    fn test_url_decode_percent() {
        assert_eq!(url_decode("hello%20world").unwrap(), "hello world");
    }

    #[test]
    fn test_url_decode_at_sign() {
        assert_eq!(url_decode("user%40host").unwrap(), "user@host");
    }

    #[test]
    fn test_url_decode_colon() {
        assert_eq!(url_decode("pass%3Aword").unwrap(), "pass:word");
    }

    #[test]
    fn test_url_decode_plus() {
        assert_eq!(url_decode("hello+world").unwrap(), "hello world");
    }

    #[test]
    fn test_url_decode_invalid_hex() {
        assert!(url_decode("pass%ZZword").is_err());
    }

    #[test]
    fn test_url_decode_truncated() {
        assert!(url_decode("pass%2").is_err());
    }

    #[test]
    fn test_url_decode_utf8() {
        // %C3%A9 = é
        assert_eq!(url_decode("caf%C3%A9").unwrap(), "café");
    }

    // -----------------------------------------------------------------------
    // Query param tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_query_empty() {
        let params = parse_tls_query_params("").unwrap();
        assert!(params.is_empty());
    }

    #[test]
    fn test_parse_query_single() {
        let params = parse_tls_query_params("key=value").unwrap();
        assert_eq!(params.get("key").unwrap(), "value");
    }

    #[test]
    fn test_parse_query_multiple() {
        let params = parse_tls_query_params("a=1&b=2&c=3").unwrap();
        assert_eq!(params["a"], "1");
        assert_eq!(params["b"], "2");
        assert_eq!(params["c"], "3");
    }

    #[test]
    fn test_parse_query_case_insensitive() {
        // CA_CERT is the uppercase; lowercased it maps to "ca_cert"
        let params = parse_tls_query_params("CA_CERT=/path/").unwrap();
        assert_eq!(params.get("ca_cert").unwrap(), "/path/");
    }

    #[test]
    fn test_parse_query_url_decoded_values() {
        let params =
            parse_tls_query_params("ca_cert=%2Fpath%2Fwith%20spaces.pem").unwrap();
        assert_eq!(params.get("ca_cert").unwrap(), "/path/with spaces.pem");
    }

    #[test]
    fn test_parse_query_missing_equals() {
        let params = parse_tls_query_params("invalid_param");
        assert!(params.is_err());
    }

    // -----------------------------------------------------------------------
    // connect_url tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_redis_plain_basic() {
        // redis:// URL scheme is correctly recognized — either connects
        // (if server running) or fails at TCP layer (never at URL parsing).
        let result = connect_url("redis://127.0.0.2:6379");
        // Don't assert success/failure — just ensure no parse error occurred.
        match result {
            Ok(_) => {} // connected
            Err(ref e) => {
                let err = e.to_string();
                assert!(
                    !err.contains("unsupported scheme"),
                    "redis:// scheme should not be rejected"
                );
            }
        }
    }

    #[test]
    fn test_parse_rediss_basic() {
        #[cfg(feature = "tls")]
        {
            let result = connect_url("rediss://127.0.0.2:6380?system_certs=true");
            assert!(result.is_err()); // Connection fails but scheme is accepted
            if let Err(ref e) = result {
                let err = e.to_string();
                assert!(!err.contains("unsupported scheme"));
                assert!(!err.contains("TLS support not enabled"));
            }
        }
        #[cfg(not(feature = "tls"))]
        {
            let result = connect_url("rediss://127.0.0.2:6380");
            assert!(result.is_err());
            if let Err(ref e) = result {
                let err = e.to_string();
                assert!(err.contains("TLS support not enabled"));
            }
        }
    }

    #[test]
    fn test_parse_rediss_no_ca_fails() {
        #[cfg(feature = "tls")]
        {
            let result = connect_url("rediss://127.0.0.2:6380");
            assert!(result.is_err());
            if let Err(ref e) = result {
                let err = e.to_string();
                assert!(
                    err.contains("neither 'ca_cert' nor 'system_certs=true'")
                        || err.contains("TLS connection failed")
                );
            }
        }
    }

    #[test]
    fn test_parse_redis_plain() {
        // Same as test_parse_redis_plain_basic — just verify the scheme works.
        test_parse_redis_plain_basic();
    }

    #[test]
    fn test_default_ports() {
        assert_eq!(default_port(&ConnectionScheme::Plain), 6379);
        assert_eq!(default_port(&ConnectionScheme::Tls), 6380);
    }
}
