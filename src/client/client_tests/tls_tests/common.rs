// Shared infrastructure for TLS integration tests.

use may::config;
use may::go;
use std::sync::Once;

use crate::tls::{
    config::{RustlsRootCerts, TlsVersion},
    TlsConfig,
};

/// One-time initialization of the may coroutine runtime.
///
/// This MUST be called once before any test that uses may-redis.
/// It starts the may scheduler by setting workers to 1.
///
/// With a SINGLE worker, the test body and the epoll loop run on the
/// same worker thread. The scheduler switches between them cooperatively.
fn init_may_runtime() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        // TLS operations (rustls handshakes) need large stacks;
        // may's default is 4KB which overflows. 64KB is plenty.
        config().set_stack_size(64 * 1024);
        config().set_workers(1);
    });
}

/// Run test logic inside the may scheduler.
///
/// CRITICAL: The test body runs inside a `go!()` on the single worker thread.
/// Inside the test body:
/// 1. `connect_tls()` spawns the epoll loop (on the SAME worker thread)
/// 2. We yield to let the epoll loop start its event loop
/// 3. `execute()` blocks waiting on a receiver
/// 4. The may scheduler switches to the epoll loop (cooperative)
/// 5. The epoll loop reads the response and writes to the receiver
/// 6. The scheduler switches back to the test body
/// 7. `execute()` returns
///
/// The key insight: `go!()` runs on the same worker thread as the epoll loop.
/// When `execute()` blocks on the receiver, the may scheduler yields to the
/// epoll loop (both are coroutines on the same worker thread). This is
/// cooperative scheduling — no OS threads blocked, just coroutine yields.
///
/// We use a channel on the MAIN thread to wait for the result:
/// - Main thread: spawn `go!()` with test body, then block on `rx.recv()`
/// - Worker thread: may scheduler runs both test body and epoll loop
/// - Main thread: receives result from channel and returns
pub(super) fn run_may<F, T>(f: F) -> T
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    init_may_runtime();

    let (tx, rx) = may::sync::spsc::channel::<T>();

    // Spawn the test body on the WORKER thread via go!().
    // Inside it, connect_tls() spawns the epoll loop on the SAME worker thread.
    // The may scheduler switches between test body and epoll loop cooperatively.
    go!(move || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f()));
        match result {
            Ok(val) => {
                tx.send(val).ok();
            }
            Err(e) => {
                let msg = if let Some(s) = e.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = e.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "unknown panic".to_string()
                };
                // Drop the channel to signal panic to the main thread
                drop(tx);
                panic!("test body panicked: {msg}");
            }
        }
    });

    // Block on the channel to wait for the test body to complete.
    // This blocks the MAIN thread, but the worker thread's may scheduler
    // is running cooperatively — the epoll loop can yield to the test body
    // and vice versa.
    rx.recv().expect("test body panicked or channel closed")
}

/// Build a TlsConfig that connects to localhost using our test CA.
///
/// IMPORTANT: Tests must yield AFTER calling connect_tls() but BEFORE
/// calling execute(), to give the epoll loop time to start. This is
/// done by wrapping the test body in a closure that yields:
///
/// ```
/// run_may(|| {
///     let client = RedisClient::connect_tls(...);
///     may::coroutine::yield_now(); // let epoll loop start
///     client.ping();
/// })
/// ```
pub(super) fn test_tls_config() -> TlsConfig {
    TlsConfig {
        root_certificates: RustlsRootCerts::Pem(vec![std::path::PathBuf::from(
            "/home/casibbald/Workspace/microscaler/may_redis/tests/tls/ca.crt",
        )]),
        server_name: "localhost".to_string(),
        min_version: TlsVersion::Tls12,
        max_version: TlsVersion::Tls13,
        client_certs: None,
        verify_server: true,
    }
}
