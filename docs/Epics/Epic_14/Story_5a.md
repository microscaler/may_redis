# Story 14.5a — TlsVersion::from_str

**Objective:** Implement `TlsVersion::from_str()` — parse "1.2" or "1.3" strings with error handling.

**Epic:** 14 — TLS and mTLS Support (Sub-story 1/4 of Story 5)

**Dependencies:** Story 14.1a (TlsVersion type exists)

## Deliverable

1. `TlsVersion::from_str(s: &str) -> Result<Self, TlsError>` — parses version strings
2. `TlsError::InvalidTlsVersion(String)` variant

## Tasks

1. Patch `src/tls/mod.rs` — add `InvalidTlsVersion(String)` to `TlsError` enum
2. Patch `src/tls/mod.rs` — implement `TlsVersion::from_str()`:
   ```rust
   pub fn from_str(s: &str) -> Result<Self, TlsError> {
       match s.trim() {
           "1.2" => Ok(Self::Tls12),
           "1.3" => Ok(Self::Tls13),
           _ => Err(TlsError::InvalidTlsVersion(format!(
               "unsupported TLS version: {s} (expected '1.2' or '1.3')"
           ))),
       }
   }
   ```
3. Add Display impl for InvalidTlsVersion
4. Run `cargo build --features tls`
5. Run `cargo test --lib --features tls`

## Verification

- `cargo build --features tls` compiles
- `cargo test --lib --features tls` — from_str tests pass
- `cargo fmt --all --check` passes
