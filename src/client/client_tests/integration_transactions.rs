#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::unit::{integration_redis_port, run_integration, shared_client};
use crate::protocol::commands::{AdminCommands, StringsCommands, TransactionsCommands};
use crate::RedisClient;

// ---------------------------------------------------------------------------
// MULTI/EXEC — Basic transaction
// ---------------------------------------------------------------------------

#[test]
fn test_integration_transaction_multi_exec() {
    run_integration(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        // Start transaction
        let _: String = client.execute(client.multi()).unwrap();

        // Queue commands (they return QUEUED)
        let _: String = client.execute(client.set("tx_key", "from_tx")).unwrap();
        let _: String = client.execute(client.set("tx_key2", "from_tx2")).unwrap();

        // Execute the transaction
        let results: Vec<String> = client.execute(client.exec()).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].to_lowercase(), "ok");
        assert_eq!(results[1].to_lowercase(), "ok");

        // Verify values were actually set
        let val: Option<String> = client.execute(client.get("tx_key")).unwrap();
        assert_eq!(val, Some("from_tx".to_string()));

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// MULTI/DISCARD — Abort transaction
// ---------------------------------------------------------------------------

#[test]
fn test_integration_transaction_discard() {
    run_integration(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client
            .execute::<()>(client.set("discard_key", "original"))
            .ok();

        // Start transaction
        let _: String = client.execute(client.multi()).unwrap();

        // Queue a SET that would overwrite the key
        let _: String = client
            .execute(client.set("discard_key", "should_not_appear"))
            .unwrap();

        // DISCARD the transaction
        let _: String = client.execute(client.discard()).unwrap();
        assert_eq!(
            client
                .execute::<Option<String>>(client.get("discard_key"))
                .unwrap()
                .unwrap(),
            "original"
        );

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// WATCH — Monitor key for changes
// ---------------------------------------------------------------------------

#[test]
fn test_integration_transaction_watch() {
    run_integration(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client
            .execute::<()>(client.set("watch_key", "base_value"))
            .ok();

        // WATCH the key
        let keys: Vec<&str> = vec!["watch_key"];
        let _: String = client.execute(client.watch(&keys)).unwrap();

        // Start transaction
        let _: String = client.execute(client.multi()).unwrap();

        // Queue a SET
        let _: String = client
            .execute(client.set("watch_key", "new_value"))
            .unwrap();

        // Execute — should succeed since no one changed the key
        let results: Vec<String> = client.execute(client.exec()).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].to_lowercase(), "ok");

        client.execute::<()>(client.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// WATCH + conflict — Transaction aborts when watched key changes
// ---------------------------------------------------------------------------

#[test]
fn test_integration_transaction_watch_conflict() {
    run_integration(|| {
        let port = integration_redis_port();
        let c1 = RedisClient::connect("127.0.0.1", port).expect("transaction client");
        let c2 = RedisClient::connect("127.0.0.1", port).expect("conflict client");
        may::coroutine::yield_now();

        c1.execute::<()>(c1.flushdb()).ok();
        c1.execute::<()>(c1.set("conflict_key", "original")).ok();

        let _: String = c1.execute(c1.watch(&["conflict_key"])).unwrap();
        let _: String = c1.execute(c1.multi()).unwrap();
        let _: String = c1.execute(c1.set("conflict_key", "from_c1")).unwrap();

        c2.execute::<()>(c2.set("conflict_key", "from_other"))
            .unwrap();

        // Redis returns RESP2 null array (*-1) when WATCH detects a conflict.
        let exec_result: Option<Vec<String>> = c1.execute(c1.exec()).unwrap();
        assert!(
            exec_result.is_none(),
            "EXEC should return null on watch conflict, got {exec_result:?}"
        );

        let val: Option<String> = c2.execute(c2.get("conflict_key")).unwrap();
        assert_eq!(val, Some("from_other".to_string()));

        c1.execute::<()>(c1.flushdb()).ok();
    });
}

// ---------------------------------------------------------------------------
// UNWATCH — Cancel watching
// ---------------------------------------------------------------------------

#[test]
fn test_integration_transaction_unwatch() {
    run_integration(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client.execute::<()>(client.set("unwatch_key", "base")).ok();

        // WATCH the key
        let keys: Vec<&str> = vec!["unwatch_key"];
        let _: String = client.execute(client.watch(&keys)).unwrap();

        // UNWATCH should clear the watch
        let _: String = client.execute(client.unwatch()).unwrap();

        // Now changes to the key won't cause EXEC to fail
        let _: () = client
            .execute(client.set("unwatch_key", "changed"))
            .unwrap();

        let _: String = client.execute(client.multi()).unwrap();
        let _: String = client
            .execute(client.set("unwatch_key", "exec_value"))
            .unwrap();

        let results: Vec<String> = client.execute(client.exec()).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].to_lowercase(), "ok");

        client.execute::<()>(client.flushdb()).ok();
    });
}
