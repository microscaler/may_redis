#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::used_underscore_items
)]
use crate::RedisClient;

use may::config;
use may::go;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::Once;

/// One-time initialization of the may coroutine runtime.
///
/// The may scheduler is lazily initialized on first call to
/// `config().set_workers()`. We initialize it once so that every
/// test thread has a valid may context before spawning coroutines.
///
/// Without this, `go!` panics on fresh std threads (e.g. CI runners)
/// because the may scheduler hasn't been started yet.
fn init_may_runtime() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        config().set_workers(2);
    });
}

/// Test that RedisClient struct is constructible
#[test]
fn test_redis_client_struct() {
    fn _assert_send_sync<T: Send + Sync>() {}
    _assert_send_sync::<RedisClient>();
}

/// Test that all Commands domain traits are implemented
#[test]
fn test_commands_trait_methods_exist() {
    fn _require_strings<T: crate::protocol::commands::StringsCommands>() {}
    fn _require_hashes<T: crate::protocol::commands::HashesCommands>() {}
    fn _require_sets<T: crate::protocol::commands::SetsCommands>() {}
    fn _require_lists<T: crate::protocol::commands::ListsCommands>() {}
    fn _require_sorted_sets<T: crate::protocol::commands::SortedSetsCommands>() {}
    fn _require_pubsub<T: crate::protocol::commands::PubsubCommands>() {}
    fn _require_transactions<T: crate::protocol::commands::TransactionsCommands>() {}
    fn _require_admin<T: crate::protocol::commands::AdminCommands>() {}
    _require_strings::<RedisClient>();
    _require_hashes::<RedisClient>();
    _require_sets::<RedisClient>();
    _require_lists::<RedisClient>();
    _require_sorted_sets::<RedisClient>();
    _require_pubsub::<RedisClient>();
    _require_transactions::<RedisClient>();
    _require_admin::<RedisClient>();
}

// ---------------------------------------------------------------------------
// Integration tests — require Redis (Docker fixture or localhost:6379)
// ---------------------------------------------------------------------------

/// Start Docker Redis on the test main thread when `test` feature is enabled.
pub(super) fn prepare_integration_tests() -> bool {
    if std::env::var("SKIP_DOCKER_TESTS").is_ok() {
        eprintln!("SKIP: SKIP_DOCKER_TESTS is set");
        return false;
    }
    #[cfg(feature = "test")]
    {
        match crate::test_fixture::ensure_started() {
            Ok(()) => true,
            Err(err) => {
                eprintln!("SKIP: {err}");
                false
            }
        }
    }
    #[cfg(not(feature = "test"))]
    {
        true
    }
}

pub(super) fn integration_redis_port() -> u16 {
    #[cfg(feature = "test")]
    if let Some(port) = crate::test_fixture::plain_redis_port() {
        return port;
    }
    6379
}

/// Returns the shared RedisClient, initializing it on first call.
pub(super) fn shared_client() -> RedisClient {
    static INIT: std::sync::Once = std::sync::Once::new();
    static CLIENT: std::sync::OnceLock<RedisClient> = std::sync::OnceLock::new();
    INIT.call_once(|| {
        let port = integration_redis_port();
        CLIENT
            .set(
                RedisClient::connect("127.0.0.1", port)
                    .expect("Redis integration fixture connection failed"),
            )
            .ok();
    });
    CLIENT.get().expect("client not initialized").clone()
}

/// Run an integration test body inside the may scheduler (skips when Docker unavailable).
pub(super) fn run_integration<F>(f: F)
where
    F: FnOnce() + Send + 'static,
{
    if !prepare_integration_tests() {
        return;
    }
    run_may(f);
}

/// Run e2e test logic inside the may scheduler.
///
/// Uses `go!` to spawn the test body as a may coroutine, then joins it.
/// The coroutine's `rx.recv()` calls cooperatively yield, letting the
/// connection-loop coroutine run and dispatch responses.
///
/// `init_may_runtime()` must be called before `go!` to ensure the may
/// scheduler is initialized. Without this, spawning coroutines on a fresh
/// std thread (as happens in CI) will panic.
pub(super) fn run_may<F, T>(f: F) -> T
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    // Ensure the may scheduler is initialized on this thread before spawning
    // any coroutines. Without this, go! panics on fresh std threads.
    init_may_runtime();

    let wrapper = Arc::new(Mutex::new(None::<T>));
    let wrapper2 = Arc::clone(&wrapper);

    let handle = go!(move || {
        let val = f();
        *wrapper2.lock().unwrap() = Some(val);
    });

    let result = handle.join();
    match result {
        Ok(()) => wrapper
            .lock()
            .unwrap()
            .take()
            .expect("test coroutine did not store result"),
        Err(e) => panic!("test coroutine panicked: {e:?}"),
    }
}
