# Story 14.1c — TlsStream + nonblock_read/write integration

**Objective:** Implement `TlsStream` struct that wraps `rustls::ClientConnection` + `may::net::TcpStream`, implements `Read`/`Write`, and exposes `inner_mut()` for epoll.

**Epic:** 14 — TLS and mTLS Support (Sub-story 3/4 of Story 1)

**Dependencies:** Story 14.1b (handshake loop compiles)

## Deliverable

1. `TlsStream` struct — two separate fields: `conn: ClientConnection`, `stream: TcpStream`
2. `impl Read for TlsStream` — delegates to `self.conn.reader().read()`
3. `impl Write for TlsStream` — delegates to `self.conn.writer().write()` + flush
4. `TlsStream::inner_mut() -> &mut TcpStream` — exposes raw stream for wait_io()
5. `TlsStream::new()` — private constructor used by TlsConnector

## Tasks

1. Patch `src/tls/mod.rs` — add `TlsStream` struct with `conn` and `stream` fields
2. Patch `src/tls/mod.rs` — implement `Read` and `Write` for `TlsStream`
3. Patch `src/tls/mod.rs` — implement `inner_mut()` and `inner()` accessors
4. Patch `src/tls/mod.rs` — add `#[cfg(test)]` tests for TlsStream existence
5. Run `cargo build --features tls`
6. Run `cargo test --lib --features tls`

## Verification

- `cargo build --features tls` compiles
- `cargo test --lib --features tls` passes
- `cargo fmt --all --check` passes
