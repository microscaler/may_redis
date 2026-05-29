# Story 14.4b — from_tls_stream_with_ssrf

**Objective:** Add `Connection::from_tls_stream_with_ssrf()` — same as `from_tls_stream()` but stores `ssrf_config`.

**Epic:** 14 — TLS and mTLS Support (Sub-story 2/4 of Story 4)

**Dependencies:** Story 14.4a (connect_tls_with_ssrf exists)

## Deliverable

1. `Connection::from_tls_stream_with_ssrf(tls_stream, ssrf_config)` — stores ssrf_config on the Connection struct

## Tasks

1. Patch `src/connection/connection.rs` — add `from_tls_stream_with_ssrf()`:
   - Same body as `from_tls_stream()` but passes `ssrf_config` to the Connection struct
   - Store in `self.ssrf_config: Some(ssrf_config)`
2. Run `cargo build --features tls`
3. Run `cargo test --lib --features tls`

## Verification

- `cargo build --features tls` compiles
- `cargo test --lib --features tls` passes
- `cargo fmt --all --check` passes
