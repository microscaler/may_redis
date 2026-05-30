#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
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
        config().set_workers(1);
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
// Integration tests — require a Redis server on localhost:6379
// ---------------------------------------------------------------------------
// Each test runs inside `run_may()` which spawns the test body as a
// coroutine via `go!` (the may crate's coroutine spawning macro). This
// ensures the may scheduler is properly initialized on the current thread
// before spawning any coroutines.
//
// CRITICAL: We reuse a SINGLE shared RedisClient across all integration
// tests. Creating a new connection per test spawns a new epoll coroutine
// that gets cancelled on drop, exhausting the may scheduler's coroutine
// pool after ~4 tests. Keeping one connection alive avoids this.
// ---------------------------------------------------------------------------

/// Returns the shared RedisClient, initializing it on first call.
pub(super) fn shared_client() -> RedisClient {
    static INIT: std::sync::Once = std::sync::Once::new();
    static CLIENT: std::sync::OnceLock<RedisClient> = std::sync::OnceLock::new();
    INIT.call_once(|| {
        CLIENT
            .set(
                RedisClient::connect("127.0.0.1", 6379)
                    .expect("Redis must be running on localhost:6379"),
            )
            .ok();
    });
    CLIENT.get().expect("client not initialized").clone()
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

    // JoinHandle::join() blocks at the coroutine level (park/unpark),
    // so it cooperatively yields while the scheduler runs the connection loop.
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

