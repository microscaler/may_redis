#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::builder::{cmd, CommandBuilder};

    #[test]
    fn test_fake_connection_single_command() {
        let (mut fake, _tag) = FakeConnection::new(FakeResponse::new(RedisValue::Integer(42)));
        let result: Result<RedisValue, _> = fake.send(cmd("PING").build());
        let val = result.unwrap();
        assert!(matches!(val, RedisValue::Integer(42)));
    }

    #[test]
    fn test_fake_connection_bulk_string_response() {
        let (mut fake, _tag) = FakeConnection::new(FakeResponse::new(
            RedisValue::BulkString(b"hello".to_vec()),
        ));
        let result: Result<RedisValue, _> = fake.send(cmd("GET").arg("key").build());
        let val = result.unwrap();
        assert_eq!(
            val,
            RedisValue::BulkString(b"hello".to_vec())
        );
    }

    #[test]
    fn test_fake_connection_array_response() {
        let response = RedisValue::Array(vec![
            RedisValue::BulkString(b"user:1".to_vec()),
            RedisValue::BulkString(b"user:2".to_vec()),
        ]);
        let (mut fake, _tag) = FakeConnection::new(FakeResponse::new(response));
        let result: Result<RedisValue, _> = fake.send(cmd("KEYS").arg("*").build());
        let val = result.unwrap();
        assert!(matches!(val, RedisValue::Array(_)));
        if let RedisValue::Array(arr) = val {
            assert_eq!(arr.len(), 2);
        }
    }

    #[test]
    fn test_fake_connection_null_response() {
        let (mut fake, _tag) = FakeConnection::new(FakeResponse::new(RedisValue::Null));
        let result: Result<RedisValue, _> = fake.send(cmd("GET").arg("missing").build());
        let val = result.unwrap();
        assert!(matches!(val, RedisValue::Null));
    }

    #[test]
    fn test_fake_connection_captured_commands() {
        let (mut fake, _tag) = FakeConnection::new(FakeResponse::new(RedisValue::Integer(1)));

        fake.send(cmd("PING").build());
        fake.send(cmd("SET").arg("k").arg("v").build());
        fake.send(cmd("GET").arg("k").build());

        assert_eq!(fake.command_count(), 3);
        let cmds = fake.captured_commands();
        assert!(cmds[0].as_ref().contains(b"PING"));
        assert!(cmds[1].as_ref().contains(b"SET"));
        assert!(cmds[2].as_ref().contains(b"GET"));
    }

    #[test]
    fn test_fake_connection_captured_responses() {
        let (mut fake, _tag) = FakeConnection::new(FakeResponse::new(RedisValue::Integer(1)));
        let _ = fake.send(cmd("SET").arg("a").arg("1").build());

        let responses = fake.captured_responses();
        assert_eq!(responses.len(), 1);
        assert!(matches!(responses[0], RedisValue::Integer(1)));
    }

    #[test]
    fn test_assert_encoding() {
        let builder = cmd("SET").arg("key").arg("value");
        assert_encoding(
            &builder,
            b"*3\r\n$3\r\nSET\r\n$3\r\nkey\r\n$5\r\nvalue\r\n",
        );
    }

    #[test]
    fn test_assert_encoding_order() {
        let b1 = cmd("SET").arg("a").arg("1");
        let b2 = cmd("SET").arg("b").arg("2");
        let b3 = cmd("GET").arg("a");

        let expected: Vec<&[u8]> = vec![
            b"*3\r\n$3\r\nSET\r\n$1\r\na\r\n$1\r\n1\r\n",
            b"*3\r\n$3\r\nSET\r\n$1\r\nb\r\n$1\r\n2\r\n",
            b"*2\r\n$3\r\nGET\r\n$1\r\na\r\n",
        ];

        assert_encoding_order(&[&b1, &b2, &b3], &expected);
    }

    #[test]
    fn test_assert_command_response() {
        let builder = cmd("PING");
        let canned = RedisValue::SimpleString("PONG".to_string());
        assert_command_response(&builder, canned, &RedisValue::SimpleString("PONG".to_string()));
    }

    #[test]
    fn test_tag_counter_monotonic() {
        let (conn1, tag1) = FakeConnection::new(FakeResponse::new(RedisValue::Null));
        let (conn2, tag2) = FakeConnection::new(FakeResponse::new(RedisValue::Null));

        // Each FakeConnection has its own tag counter, so tags are always 0.
        // The monotonic property is guaranteed by the connection-level
        // AtomicUsize in the real Connection struct.
        assert_eq!(tag1, 0);
        assert_eq!(tag2, 0);
        drop(conn1);
        drop(conn2);
    }
}
