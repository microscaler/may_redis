#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::unit::{run_may, shared_client};
use crate::protocol::commands::{AdminCommands, PubsubCommands, StringsCommands};

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_ping() {
    run_may(|| {
        let client = shared_client();
        let result = client.ping();
        assert_eq!(result.unwrap(), "PONG");
        client.execute::<String>(client.flushdb()).ok();
    });
}

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_set_get() {
    run_may(|| {
        let client = shared_client();
        client
            .execute::<()>(client.set("test_key", "hello"))
            .unwrap();
        let result: Option<String> = client.execute(client.get("test_key")).unwrap();
        assert_eq!(result, Some("hello".to_string()));
        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_incr() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        let val: i64 = client.execute(client.incr("counter")).unwrap();
        assert_eq!(val, 1);

        let val: i64 = client.execute(client.incr("counter")).unwrap();
        assert_eq!(val, 2);

        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_exists_del() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client.execute::<()>(client.set("key1", "val1")).unwrap();
        let exists: bool = client.execute(client.exists("key1")).unwrap();
        assert!(exists);

        let exists: bool = client.execute(client.exists("missing")).unwrap();
        assert!(!exists);

        client.execute::<()>(client.del("key1")).unwrap();
        let exists: bool = client.execute(client.exists("key1")).unwrap();
        assert!(!exists);

        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_dbsize() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        let size: usize = client.execute(client.dbsize()).unwrap();
        assert_eq!(size, 0);

        client.execute::<()>(client.set("a", "1")).unwrap();
        client.execute::<()>(client.set("b", "2")).unwrap();
        let size: usize = client.execute(client.dbsize()).unwrap();
        assert_eq!(size, 2);

        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_set_ex_ttl() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client
            .execute::<()>(client.set_ex("ttl_key", "val", 60))
            .unwrap();
        let result: Option<String> = client.execute(client.get("ttl_key")).unwrap();
        assert_eq!(result, Some("val".to_string()));

        let ttl: i64 = client.execute(client.ttl("ttl_key")).unwrap();
        assert!(ttl > 0 && ttl <= 60);

        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_keys() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client.execute::<()>(client.set("user:1", "alice")).unwrap();
        client.execute::<()>(client.set("user:2", "bob")).unwrap();
        client.execute::<()>(client.set("other:1", "x")).unwrap();

        let keys: Vec<String> = client.execute(client.keys("user:*")).unwrap();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"user:1".to_string()));
        assert!(keys.contains(&"user:2".to_string()));

        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_send_sync_clone() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        let cloned = client.clone();
        cloned
            .execute::<()>(cloned.set("clone_test", "works"))
            .unwrap();
        let val: Option<String> = client.execute(cloned.get("clone_test")).unwrap();
        assert_eq!(val, Some("works".to_string()));

        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_error_propagation() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client
            .execute::<()>(client.set("str_key", "not_a_number"))
            .unwrap();
        let result: Result<i64, _> = client.execute(client.incr("str_key"));
        assert!(result.is_err());

        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_pipeline() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        // Build a pipeline with SET, SET, SET, GET
        let mut pipeline = client.pipeline();
        pipeline.add(client.set("pip:1", "a"));
        pipeline.add(client.set("pip:2", "b"));
        pipeline.add(client.set("pip:3", "c"));
        pipeline.add(client.get("pip:1"));

        let results: ((), (), (), Option<String>) = pipeline.execute().unwrap();
        assert_eq!(results, ((), (), (), Some("a".to_string())));

        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_concurrent() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        let c1 = client.clone();
        let c2 = client.clone();

        c1.execute::<()>(c1.set("concurrent:a", "1")).unwrap();
        c2.execute::<()>(c2.set("concurrent:b", "2")).unwrap();

        let v1: Option<String> = c1.execute(c1.get("concurrent:a")).unwrap();
        let v2: Option<String> = c2.execute(c2.get("concurrent:b")).unwrap();

        assert_eq!(v1, Some("1".to_string()));
        assert_eq!(v2, Some("2".to_string()));

        let keys: Vec<String> = client.execute(client.keys("concurrent:*")).unwrap();
        assert_eq!(keys.len(), 2);

        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_concurrent_requests() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client
            .execute::<()>(client.set("concurrent:x", "alpha"))
            .unwrap();
        client
            .execute::<()>(client.set("concurrent:y", "beta"))
            .unwrap();
        client
            .execute::<()>(client.set("concurrent:z", "gamma"))
            .unwrap();

        let c1 = client.clone();
        let c2 = client.clone();
        let c3 = client.clone();

        let v1: Option<String> = c1.execute(c1.get("concurrent:x")).unwrap();
        let v2: Option<String> = c2.execute(c2.get("concurrent:y")).unwrap();
        let v3: Option<String> = c3.execute(c3.get("concurrent:z")).unwrap();

        assert_eq!(v1, Some("alpha".to_string()));
        assert_eq!(v2, Some("beta".to_string()));
        assert_eq!(v3, Some("gamma".to_string()));

        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_concurrent_pipelines() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        let c1 = client.clone();
        let c2 = client.clone();

        let mut pipe1 = c1.pipeline();
        pipe1.add(c1.set("cp1:a", "val1a"));
        pipe1.add(c1.set("cp1:b", "val1b"));
        pipe1.add(c1.get("cp1:a"));
        let ((), (), got_a1): ((), (), Option<String>) = pipe1.execute().unwrap();

        let mut pipe2 = c2.pipeline();
        pipe2.add(c2.set("cp2:a", "val2a"));
        pipe2.add(c2.set("cp2:b", "val2b"));
        pipe2.add(c2.get("cp2:b"));
        let ((), (), got_p2_b): ((), (), Option<String>) = pipe2.execute().unwrap();

        assert_eq!(got_a1, Some("val1a".to_string()));
        assert_eq!(got_p2_b, Some("val2b".to_string()));

        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_request_ordering() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        let mut pipe = client.pipeline();
        for i in 0..50 {
            pipe.add(client.set(format!("order:{i}"), i.to_string()));
        }
        let results: Vec<()> = pipe.execute().unwrap();
        assert_eq!(results.len(), 50);

        let mut pipe = client.pipeline();
        for i in 0..50 {
            pipe.add(client.get(format!("order:{i}")));
        }
        let results: Vec<Option<String>> = pipe.execute().unwrap();
        for (i, val) in results.iter().enumerate() {
            assert_eq!(val, &Some(i.to_string()), "order:{i} mismatch");
        }

        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_response_correlation() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        for i in 0..10 {
            client
                .execute::<()>(client.set(format!("rc:{i}"), format!("resp-{i}")))
                .unwrap();
        }

        for i in 0..10 {
            let c = client.clone();
            let expected = format!("resp-{i}");
            let v: Option<String> = c.execute(c.get(format!("rc:{i}"))).unwrap();
            assert_eq!(v, Some(expected), "correlation failed for key {i}");
        }

        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_server_error_propagation() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client
            .execute::<()>(client.set("err:str", "not_a_number"))
            .unwrap();

        let result: Result<i64, _> = client.execute(client.incr("err:str"));
        assert!(result.is_err());

        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_wrong_type_extraction() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        let result: Result<String, _> = client.execute(client.dbsize());
        assert!(result.is_err(), "DBSIZE→String should error");

        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_empty_pipeline() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        let mut pipe = client.pipeline();
        let result: Result<Vec<()>, _> = pipe.execute();
        assert_eq!(
            result.unwrap(),
            Vec::<()>::new(),
            "empty pipeline should return empty vec"
        );

        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_null_response_handling() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        let result: Result<Option<String>, _> = client.execute(client.get("missing_key"));
        assert_eq!(result.unwrap(), None, "GET missing key should return None");

        client
            .execute::<()>(client.set("null_test:exists", "val"))
            .unwrap();
        let result: Result<Option<String>, _> = client.execute(client.get("null_test:exists"));
        assert_eq!(result.unwrap(), Some("val".to_string()));

        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_redis_server_error_value() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client
            .execute::<()>(client.set("str_key2", "not_a_number"))
            .unwrap();
        let result: Result<i64, _> = client.execute(client.incr("str_key2"));
        assert!(
            result.is_err(),
            "INCR on string should fail, got: {result:?}"
        );

        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_set_get_ex() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client
            .execute::<()>(client.set_ex("ex_key", "ex_val", 60))
            .unwrap();
        let result: Option<String> = client.execute(client.get("ex_key")).unwrap();
        assert_eq!(result, Some("ex_val".to_string()));

        let ttl: i64 = client.execute(client.ttl("ex_key")).unwrap();
        assert!(ttl > 0 && ttl <= 60);

        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_del() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client
            .execute::<()>(client.set("del_key", "del_val"))
            .unwrap();
        let exists: bool = client.execute(client.exists("del_key")).unwrap();
        assert!(exists);

        client.execute::<()>(client.del("del_key")).unwrap();
        let exists: bool = client.execute(client.exists("del_key")).unwrap();
        assert!(!exists);

        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_expire() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        client
            .execute::<()>(client.set("exp_key", "exp_val"))
            .unwrap();
        let exists: bool = client.execute(client.exists("exp_key")).unwrap();
        assert!(exists);

        client.execute::<()>(client.expire("exp_key", 60)).unwrap();
        let ttl: i64 = client.execute(client.ttl("exp_key")).unwrap();
        assert!(ttl > 0 && ttl <= 60);

        client.execute::<()>(client.flushdb()).ok();
    });
}

#[test]
#[ignore = "requires live Redis server"]
fn test_integration_publish() {
    run_may(|| {
        let client = shared_client();
        client.execute::<()>(client.flushdb()).ok();

        let result: i64 = client
            .execute(client.publish("test_channel", "test_message"))
            .unwrap();
        // No subscribers, so 0
        assert_eq!(result, 0);

        client.execute::<()>(client.flushdb()).ok();
    });
}
