// TLS integration tests — may runtime + bollard-managed Redis-TLS containers.
//
// Requires Docker and `--features tls,test`.
// Run: cargo test --features tls,test tls_tests -- --test-threads=1

#[cfg(feature = "tls")]
mod common;

#[cfg(feature = "tls")]
mod handshake;

#[cfg(feature = "tls")]
mod connect_tls;

#[cfg(feature = "tls")]
mod from_tls_stream;

#[cfg(feature = "tls")]
mod execute_timeout;

#[cfg(feature = "tls")]
mod connection_methods;

#[cfg(feature = "tls")]
mod tls_stream;

#[cfg(feature = "tls")]
mod tcp_connector;
