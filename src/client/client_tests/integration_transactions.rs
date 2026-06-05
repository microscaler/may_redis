#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::unit::{run_integration, shared_client};
use crate::protocol::commands::{AdminCommands, StringsCommands, TransactionsCommands};

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
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client
            .execute::<()>(client.set("conflict_key", "original"))
            .ok();

        // First client watches and starts transaction
        let c1 = client.clone();
        let _: String = c1.execute(client.watch(&["conflict_key"])).unwrap();
        let _: String = c1.execute(c1.multi()).unwrap();
        let _: String = c1.execute(c1.set("conflict_key", "from_c1")).unwrap();

        // Second client changes the watched key
        let _: () = client
            .execute(client.set("conflict_key", "from_other"))
            .unwrap();

        // First client tries to execute — should fail (nil/None)
        let result: Result<Option<String>, _> = c1.execute(c1.exec());
        assert!(
            result.is_ok(),
            "exec should succeed (returns None on conflict)"
        );
        // Redis returns nil (null) when watch conflict occurs
        let exec_result = result.unwrap();
        assert!(
            exec_result.is_none()
                || exec_result.iter().any(|r| r == "nil" || r.is_empty()),
            "EXEC should return nil on watch conflict"
        );

        // Verify key wasn't changed by c1
        let val: Option<String> = client.execute(client.get("conflict_key")).unwrap();
        assert_eq!(val, Some("from_other".to_string()));

        client.execute::<()>(client.flushdb()).ok();
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
