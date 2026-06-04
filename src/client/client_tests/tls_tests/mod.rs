// TLS integration tests — require may runtime + Redis-TLS on localhost:6380
//
// All tests are #[ignore] and require a live Redis-TLS server.
// Start with:
//   docker run -d --name may-redis-tls-test -p 6380:6380 \
//     -v ./tests/tls:/tls redis:7-alpine redis-server /tls/redis-tls.conf
//
// Run with: cargo test --features tls tls_tests -- --ignored --test-threads=1

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
