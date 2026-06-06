//! Re-export the in-crate Docker fixture for integration tests.
//!
//! Run with `--features test` so `may_redis::test_fixture` is compiled.
#![cfg(feature = "test")]
pub use may_redis::test_fixture::*;
