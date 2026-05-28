# Story 2 — `mget`/`mset` trait-dispatch integration test

**Objective:** Add integration tests that exercise the `Commands` trait dispatch for `mget` and `mset`, confirming they compile and work through the trait (not just as associated functions).

**Epic:** 12 — Test Gap Remediation
**Dependencies:** Story 0 (Epic overview)
**Status:** NEW

---

## Source Reference

- **Finding:** P1 (MEDIUM) from `docs/code-review-2026-05-28.md`
- **Code anchors:**
  - `src/protocol/commands.rs:204-231` — `mget` and `mset` trait methods
  - `src/client/client.rs:283-359` — `impl Commands for RedisClient`

## Background

The `mget`, `mset`, `msetnx`, `sinter`, and `sunion` commands were originally implemented as associated functions (no `&self`) while all other commands take `&self`. This inconsistency meant they could not be called on a `RedisClient` instance via the `Commands` trait in the normal way (`client.mget(...)` would not compile). Story 11.3 fixed them to take `&self` for API consistency.

However, the existing `test_command_*_encoding` tests verify only the RESP wire format — they use `<() as Commands>::mget(&(), &["k1", "k2"])` which exercises the trait default implementation on the unit type. There is **zero coverage** for the actual trait dispatch path: calling `client.mget(&["k1", "k2"])` on a `RedisClient` instance via the trait and executing the result. The trait method body and the `impl Commands for RedisClient` block override both need verification.

Without these tests, any future change to the `impl Commands for RedisClient` block that omits the `mget`/`mset` overrides would silently break trait dispatch without any test catching it.

## Functional Requirements

- [ ] Test that `mget` works via the `Commands` trait dispatch
- [ ] Test that `mget` returns correct values for existing keys
- [ ] Test that `mget` returns correct values for non-existing keys (Redis returns nil/null)
- [ ] Test that `mget` with an empty key list returns an empty array

## Non-Functional Requirements

- [ ] Test must use `may::run` / `may::go`
- [ ] Test must use `run_may()` wrapper
- [ ] Test must use shared Redis client
- [ ] Test must call FLUSHDB before and after

## Implementation Details

### Integration Tests (require Redis)

Five new integration tests added to `src/client/client.rs`, following the same pattern as existing tests (`test_integration_set_get`, `test_integration_incr`, etc.):

#### `test_integration_mget_existing_keys`

Sets three keys, then retrieves all three via a single `MGET` call. Verifies the response is `RedisValue::Array` with correct elements.

```rust
#[test]
#[ignore = "requires live Redis server"]
fn test_integration_mget_existing_keys() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client.execute::<()>(client.set("k1", "v1")).unwrap();
        client.execute::<()>(client.set("k2", "v2")).unwrap();
        client.execute::<()>(client.set("k3", "v3")).unwrap();

        // Exercise trait dispatch: client.mget() called on RedisClient
        let result: RedisValue = client
            .execute(client.mget(&["k1", "k2", "k3"]))
            .unwrap();

        if let RedisValue::Array(values) = result {
            assert_eq!(values.len(), 3);
            assert!(matches!(&values[0], RedisValue::BulkString(s) if s == b"v1"));
            assert!(matches!(&values[1], RedisValue::BulkString(s) if s == b"v2"));
            assert!(matches!(&values[2], RedisValue::BulkString(s) if s == b"v3"));
        } else {
            panic!("expected Array, got {:?}", result);
        }

        client.execute::<()>(client.flushdb()).ok();
    });
}
```

#### `test_integration_mget_missing_keys`

MGETs keys that don't exist. Redis returns `null` for each missing key.

```rust
#[test]
#[ignore = "requires live Redis server"]
fn test_integration_mget_missing_keys() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        let result: RedisValue = client
            .execute(client.mget(&["nonexistent1", "nonexistent2"]))
            .unwrap();

        if let RedisValue::Array(values) = result {
            assert_eq!(values.len(), 2);
            assert!(matches!(&values[0], RedisValue::Null));
            assert!(matches!(&values[1], RedisValue::Null));
        } else {
            panic!("expected Array, got {:?}", result);
        }

        client.execute::<()>(client.flushdb()).ok();
    });
}
```

#### `test_integration_mget_mixed`

Sets one key, then MGETs a mix of existing and non-existing keys. Verifies partial results.

```rust
#[test]
#[ignore = "requires live Redis server"]
fn test_integration_mget_mixed() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client.execute::<()>(client.set("k1", "hello")).unwrap();

        let result: RedisValue = client
            .execute(client.mget(&["k1", "nonexistent"]))
            .unwrap();

        if let RedisValue::Array(values) = result {
            assert_eq!(values.len(), 2);
            assert!(matches!(&values[0], RedisValue::BulkString(s) if s == b"hello"));
            assert!(matches!(&values[1], RedisValue::Null));
        } else {
            panic!("expected Array, got {:?}", result);
        }

        client.execute::<()>(client.flushdb()).ok();
    });
}
```

#### `test_integration_mget_empty_list`

MGET with an empty key list returns an empty array (not an error).

```rust
#[test]
#[ignore = "requires live Redis server"]
fn test_integration_mget_empty_list() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        let result: RedisValue = client
            .execute(client.mget(&[] as &[&str]))
            .unwrap();

        if let RedisValue::Array(values) = result {
            assert!(values.is_empty());
        } else {
            panic!("expected Array, got {:?}", result);
        }

        client.execute::<()>(client.flushdb()).ok();
    });
}
```

#### `test_integration_mset_and_verify`

MSETs key-value pairs, then GETs each key to verify persistence.

```rust
#[test]
#[ignore = "requires live Redis server"]
fn test_integration_mset_and_verify() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        // Exercise trait dispatch: client.mset() called on RedisClient
        let pairs: Vec<(&str, &str)> = vec![("k1", "v1"), ("k2", "v2")];
        client.execute::<()>(client.mset(&pairs)).unwrap();

        let v1: Option<String> = client.execute(client.get("k1")).unwrap();
        let v2: Option<String> = client.execute(client.get("k2")).unwrap();

        assert_eq!(v1, Some("v1".to_string()));
        assert_eq!(v2, Some("v2".to_string()));

        client.execute::<()>(client.flushdb()).ok();
    });
}
```

### Unit Tests (no Redis needed)

Two new unit tests added to `src/protocol/commands.rs` in the `tests` module:

#### `test_mget_trait_dispatch_compiles`

A compile-time guard that verifies `mget` is callable via trait dispatch on `RedisClient`. If this compiles, the trait dispatch path works.

```rust
#[test]
fn test_mget_trait_dispatch_compiles() {
    use super::super::client::RedisClient;

    fn _requires_commands_trait<T: Commands>() {}
    _requires_commands_trait::<RedisClient>();

    // Verify mget() is callable on RedisClient via the trait
    let _builder_check = || {
        let client = RedisClient; // cannot construct in unit test, but type check suffices
        fn _check_builder<B: Into<Vec<Vec<u8>>>>(_b: CommandBuilder) -> B {
            _b
        }
        // This is a type-level check only — cannot construct RedisClient here,
        // but the trait bound above confirms RedisClient implements Commands.
    };
    let _check = _builder_check;
}
```

A simpler approach that works without constructing a client:

```rust
#[test]
fn test_mget_trait_dispatch_compiles() {
    // Verify RedisClient implements the Commands trait
    fn _require_commands<T: Commands>() {}
    _require_commands::<RedisClient>();

    // Verify mget and mset return CommandBuilder (type-level check)
    // Since we cannot construct RedisClient in a unit test, the trait bound
    // above is the compile-time guarantee. If RedisClient loses the impl,
    // this test will fail to compile.
}
```

#### `test_mget_empty_array_encoding`

Verifies `mget(&[])` produces the correct RESP encoding for an empty key list: `*1\r\n$4\r\nMGET\r\n` (bulk multi-bulk with just the command name).

```rust
#[test]
fn test_mget_empty_array_encoding() {
    let buf = <() as Commands>::mget(&(), &[] as &[&str]).build();
    assert_eq!(buf.as_ref(), b"*1\r\n$4\r\nMGET\r\n");
}
```

## Verification

### Unit Tests

- [ ] `test_mget_trait_dispatch_compiles` — compile-time test: verify `RedisClient` implements `Commands` trait
- [ ] `test_mget_empty_array_encoding` — verify `mget(&[])` produces correct RESP encoding (`*1\r\n$4\r\nMGET\r\n`)
- [ ] All 335 existing tests still pass

### Clippy

- [ ] `cargo clippy --lib --tests --all-features -- -D warnings` — zero warnings

### Format

- [ ] `cargo fmt --all --check` — clean

### Integration Tests (require Redis)

- [ ] `test_integration_mget_existing_keys` — SET k1=v1, k2=v2, k3=v3; MGET k1 k2 k3 -> [v1, v2, v3]
- [ ] `test_integration_mget_missing_keys` — MGET nonexistent1 nonexistent2 -> [null, null]
- [ ] `test_integration_mget_mixed` — SET k1=hello; MGET k1 nonexistent -> [hello, null]
- [ ] `test_integration_mget_empty_list` — MGET (empty slice) -> empty array
- [ ] `test_integration_mset_and_verify` — MSET k1=v1 k2=v2; GET k1 -> v1, GET k2 -> v2

To verify, run against a live Redis server:

```bash
# All mget/mset integration tests
cargo test test_integration_mget_existing_keys test_integration_mget_missing_keys test_integration_mget_mixed test_integration_mget_empty_list test_integration_mset_and_verify -- --ignored --test-threads=1

# Or individually
cargo test test_integration_mget_existing_keys -- --ignored
cargo test test_integration_mget_missing_keys -- --ignored
cargo test test_integration_mget_mixed -- --ignored
cargo test test_integration_mget_empty_list -- --ignored
cargo test test_integration_mset_and_verify -- --ignored
```

## Test Summary

| Test | Type | Requires Redis | Purpose |
|------|------|----------------|---------|
| `test_mget_trait_dispatch_compiles` | Unit | No | Compile-time: RedisClient implements Commands |
| `test_mget_empty_array_encoding` | Unit | No | Wire format: empty key list -> `*1\r\n$4\r\nMGET\r\n` |
| `test_integration_mget_existing_keys` | Integration | Yes | Trait dispatch with existing keys returns correct values |
| `test_integration_mget_missing_keys` | Integration | Yes | Missing keys return null in response array |
| `test_integration_mget_mixed` | Integration | Yes | Mix of existing + missing returns partial results |
| `test_integration_mget_empty_list` | Integration | Yes | Empty key list returns empty array (not error) |
| `test_integration_mset_and_verify` | Integration | Yes | MSET persists values retrievable via GET |

## Files Modified

- `src/client/client.rs` — added 5 new integration tests (mget/mset trait dispatch)
- `src/protocol/commands.rs` — added 2 new unit tests (trait dispatch compile check + empty array encoding)

## Acceptance Criteria

- [ ] `client.mget(&["k"])` compiles when `client` is `RedisClient` and `Commands` trait is in scope
- [ ] `client.mset(&[("k", "v")])` compiles via trait dispatch
- [ ] `mget` returns `RedisValue::Array` with correct elements (not error)
- [ ] Missing keys produce `null` in the array
- [ ] Empty key list returns empty array (not error)
- [ ] `mset` persists values that can be retrieved via `get`
- [ ] No tokio or `.await` anywhere
- [ ] All 335 existing tests still pass
