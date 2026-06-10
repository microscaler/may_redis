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
pub(super) fn init_may_runtime() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        // The may config is GLOBAL and effectively first-writer-wins once
        // the scheduler starts. Every test harness in this crate must call
        // THIS function instead of setting its own values — two harnesses
        // racing with different worker counts/stack sizes makes full-suite
        // runs nondeterministic.
        config().set_workers(2);
        // rustls handshakes are stack-hungry; may's small default stack
        // overflows (SIGSEGV) under TLS tests.
        config().set_stack_size(64 * 1024);
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
///
/// Initialization is guarded by a `may` mutex, NOT `std::sync::Once`: this
/// function runs inside coroutines and `connect` parks its coroutine. A
/// `Once` would block the worker *thread* in concurrent callers — with a
/// single may worker that deadlocks the whole scheduler.
pub(super) fn shared_client() -> RedisClient {
    static CLIENT: std::sync::OnceLock<RedisClient> = std::sync::OnceLock::new();
    static INIT_LOCK: std::sync::OnceLock<may::sync::Mutex<()>> =
        std::sync::OnceLock::new();

    let _guard = INIT_LOCK
        .get_or_init(|| may::sync::Mutex::new(()))
        .lock()
        .expect("shared_client init lock poisoned");
    if let Some(client) = CLIENT.get() {
        return client.clone();
    }
    let port = integration_redis_port();
    let client = RedisClient::connect("127.0.0.1", port)
        .expect("Redis integration fixture connection failed");
    CLIENT.set(client.clone()).ok();
    client
}

/// Run an integration test body inside the may scheduler (skips when Docker unavailable).
///
/// Integration tests share one `shared_client()` and one Redis DB, and call
/// FLUSHDB for isolation — so they must not run concurrently. The guard is
/// held on the libtest thread (not a may worker), serializing test bodies
/// without affecting the coroutine scheduler.
pub(super) fn run_integration<F>(f: F)
where
    F: FnOnce() + Send + 'static,
{
    static ISOLATION: std::sync::Mutex<()> = std::sync::Mutex::new(());

    if !prepare_integration_tests() {
        return;
    }
    let _guard = ISOLATION
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
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
        Err(e) => {
            // Downcast the panic payload so the real assertion message is
            // visible instead of an opaque `Any { .. }`.
            let msg = e
                .downcast_ref::<&str>()
                .map(|s| (*s).to_string())
                .or_else(|| e.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| format!("{e:?}"));
            panic!("test coroutine panicked: {msg}");
        }
    }
}
