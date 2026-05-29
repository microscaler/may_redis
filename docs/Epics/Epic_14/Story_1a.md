# Story 14.1a — TLS Cargo feature + type definitions

**Objective:** Add the `tls` Cargo feature with dependencies, create `src/tls/mod.rs` with type definitions only (no logic yet).

**Epic:** 14 — TLS and mTLS Support (Sub-story 1/4)

**Dependencies:** None

## Deliverable

1. Add `tls` feature and optional deps to `Cargo.toml`
2. Add `#[cfg(feature = "tls")] pub mod tls;` to `src/lib.rs`
3. Create `src/tls/mod.rs` with all type definitions (no functions that do work):
   - `TlsVersion` enum (Tls12, Tls13)
   - `TlsError` enum (Config, HandshakeTimeout, Handshake, Verification)
   - `RustlsRootCerts` enum (WebPkiRoots, Pem, Der)
   - `TlsConfig` struct with fields
   - `SkipVerifier` struct (skeleton, no impl yet)
4. Zero logic implementations — types only

## Tasks

1. Patch `Cargo.toml` — add `[features]` entry `tls = ["dep:rustls", "dep:rustls-pemfile", "dep:webpki-roots"]` and optional deps section
2. Patch `Cargo.toml` — expand `rustls` features to include `std`, `tls12`
3. Patch `src/lib.rs` — add `#[cfg(feature = "tls")] pub mod tls;`
4. Write `src/tls/mod.rs` — all type definitions (enums, structs, Default impls)
5. Run `cargo build` (no features) — must compile with zero TLS deps
6. Run `cargo build --features tls` — must compile

## Verification

- `cargo build` compiles with zero TLS deps
- `cargo build --features tls` compiles
- `cargo test --lib` passes
- `cargo fmt --all --check` passes
