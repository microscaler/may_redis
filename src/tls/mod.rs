// TLS module for may-redis.
//
// Provides encrypted Redis connections via rustls + ring.
//
// Module structure:
// - `config` — TlsVersion, RustlsRootCerts, ClientCerts
// - `connector` — TlsError, TlsConfig, TlsStream, TlsConnector, SkipVerifier

pub mod config;
pub mod connector;

pub use connector::{TlsConfig, TlsConnector, TlsError, TlsStream};
