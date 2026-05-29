# Story 14.3a — parse_tls_query_params

**Objective:** Implement `parse_tls_query_params()` — split query string into HashMap of decoded key=value pairs.

**Epic:** 14 — TLS and mTLS Support (Sub-story 1/4 of Story 3)

**Dependencies:** Story 14.1d (connect_tls wiring)

## Deliverable

1. `parse_tls_query_params(query: &str) -> Result<HashMap<String, String>, RedisError>`
2. Splits on `&` for key=value pairs
3. URL-decodes each key and value using existing `url_decode()` helper
4. Handles missing `=` (treats as key with empty value)
5. Returns `Parse` error for invalid percent-encoding

## Tasks

1. Patch `src/client/client.rs` — add `parse_tls_query_params()` function after `url_decode()`
   - Split query on `&`
   - Split each pair on `=`
   - url_decode both key and value
   - Return HashMap<String, String>
2. Add unit tests for parse_tls_query_params:
   - `test_parse_query_basic` — "system_certs=true&verify=false" → {system_certs: "true", verify: "false"}
   - `test_parse_query_ampersand` — "a=1&b=2&c=3" → 3 pairs
   - `test_parse_query_no_value` — "foo=" → {foo: ""}
   - `test_parse_query_empty` — "" → empty HashMap
   - `test_parse_query_invalid_encoding` — "ca=%GG" → Parse error
3. Run `cargo build --features tls`
4. Run `cargo test --lib --features tls`

## Verification

- `cargo build --features tls` compiles
- `cargo test --lib --features tls` — all parse_query tests pass
- `cargo fmt --all --check` passes
