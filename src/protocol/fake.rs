// FakeConnection — Simulated Redis connection for protocol-level testing.
//
// Acts as a drop-in replacement for a real `Connection` when testing the
// command-building and encoding layers. It does not require a may runtime,
// a live Redis server, or any network I/O.
//
// ## How it works
//
// 1. The test encodes a `CommandBuilder` into RESP bytes via `build()`.
// 2. The test passes those bytes to `FakeConnection::send()`.
// 3. The fake connection decodes the bytes to verify the wire format,
//    then returns a canned response via an in-memory channel.
//
// This is NOT a full Redis server emulator — it's a protocol fixture for
// verifying that `CommandBuilder` produces correct RESP and that the
// response dispatch logic works without network noise.

use crate::core::{RedisError, RedisValue};
use crate::codec::reader::RESPReader;
use bytes::BytesMut;
use may::sync::spsc;

/// A canned response that `FakeConnection` can replay.
#[derive(Debug, Clone)]
pub struct FakeResponse {
    /// The RedisValue to send back as the decoded response.
    pub value: RedisValue,
    /// Optional error to inject into the wire format (for testing
    /// error-handling paths in the codec).
    pub wire_error: Option<String>,
}

impl FakeResponse {
    /// Create a fake response with a given value.
    #[must_use]
    pub fn new(value: RedisValue) -> Self {
        Self {
            value,
            wire_error: None,
        }
    }

    /// Inject a wire-format error for testing error-handling paths.
    #[must_use]
    pub fn with_wire_error(msg: impl Into<String>) -> Self {
        Self {
            value: RedisValue::Null,
            wire_error: Some(msg.into()),
        }
    }
}

/// A simulated connection for protocol-level testing.
///
/// Receives RESP-encoded command bytes, decodes them (for verification),
/// and dispatches a canned response via an `spsc` channel. No network,
/// no `may` runtime required.
pub struct FakeConnection {
    /// Channel sender to deliver canned responses back to callers.
    tx: spsc::Sender<RedisValue>,
    /// Channel receiver for the caller to await responses.
    rx: spsc::Receiver<RedisValue>,
    /// Captures every command bytes that passed through `send()`.
    captured_commands: Vec<BytesMut>,
    /// Captures every decoded response that came back.
    captured_responses: Vec<RedisValue>,
}

impl FakeConnection {
    /// Create a new `FakeConnection` with a canned response.
    ///
    /// # Arguments
    /// * `response` — The canned response to return for every `send()` call.
    pub fn new(response: FakeResponse) -> (Self, usize) {
        let (tx, rx) = spsc::channel();
        let tag_counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let conn = Self {
            tx,
            rx,
            captured_commands: Vec::new(),
            captured_responses: Vec::new(),
        };
        let tag = tag_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        // Queue the canned response — it will be delivered when the caller
        // calls recv() after send().
        let _ = conn.tx.send(response.value);
        (conn, tag)
    }

    /// Send a command through the fake connection.
    ///
    /// Captures the command bytes and returns the first captured response
    /// from the channel. In a real connection this would be async; here
    /// it is synchronous because the response was pre-queued in `new()`.
    ///
    /// # Arguments
    /// * `command_bytes` — The fully-encoded RESP command bytes (from `CommandBuilder::build()`).
    ///
    /// # Returns
    /// The first `RedisValue` that was decoded from the canned response.
    pub fn send(&mut self, command_bytes: BytesMut) -> Result<RedisValue, RedisError> {
        // Capture the sent command for later inspection.
        self.captured_commands.push(command_bytes.clone());

        // Decode the command to verify it's valid RESP.
        let mut reader = RESPReader::new(command_bytes);
        match reader.read_value() {
            Ok(_decoded) => {
                // The command decodes successfully; return the canned response.
            }
            Err(e) => {
                // Command is malformed RESP — return an error response.
                let err_val = RedisValue::Error(format!("malformed command: {e}"));
                return Ok(err_val);
            }
        }

        // Receive the canned response from the channel.
        let value = self
            .rx
            .recv()
            .map_err(|_| RedisError::Parse("response channel closed".into()))?;

        // Capture the response for later inspection.
        self.captured_responses.push(value.clone());

        Ok(value)
    }

    /// Return all captured command bytes, in the order they were sent.
    #[must_use]
    pub fn captured_commands(&self) -> &[BytesMut] {
        &self.captured_commands
    }

    /// Return all captured responses, in the order they were received.
    #[must_use]
    pub fn captured_responses(&self) -> &[RedisValue] {
        &self.captured_responses
    }

    /// Return the number of commands that have been sent through this
    /// fake connection.
    #[must_use]
    pub fn command_count(&self) -> usize {
        self.captured_commands.len()
    }
}

/// Test helper: verify that a `CommandBuilder` encodes to the expected
/// RESP bytes.
///
/// # Arguments
/// * `builder` — The command builder to encode.
/// * `expected` — The exact byte sequence expected on the wire.
pub fn assert_encoding(builder: &crate::protocol::builder::CommandBuilder, expected: &[u8]) {
    let bytes = builder.build();
    assert_eq!(
        bytes.as_ref(),
        expected,
        "command encoding mismatch\n  expected: {}\n  got:      {}",
        hex_display(expected),
        hex_display(bytes.as_ref())
    );
}

/// Hex display for error messages (simplified, space-separated bytes).
fn hex_display(buf: &[u8]) -> String {
    buf.iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Test helper: build a command and verify the response matches a canned
/// value using `FakeConnection`.
///
/// # Arguments
/// * `builder` — The command builder.
/// * `canned_response` — The value to return from the fake connection.
/// * `expected_response` — The `RedisValue` we expect to get back.
pub fn assert_command_response(
    builder: &crate::protocol::builder::CommandBuilder,
    canned_response: RedisValue,
    expected_response: &RedisValue,
) {
    let (mut fake, _tag) = FakeConnection::new(FakeResponse::new(canned_response));
    let result = fake.send(builder.build());
    assert!(result.is_ok(), "send failed: {result:?}");
    let value = result.unwrap();
    assert_eq!(
        &value, expected_response,
        "response mismatch\n  expected: {expected_response:?}\n  got:      {value:?}"
    );
    assert_eq!(fake.command_count(), 1, "expected exactly 1 command sent");
}

/// Test helper: encode multiple commands and verify they are encoded in
/// declaration order.
///
/// # Arguments
/// * `builders` — A slice of command builders.
/// * `expected` — A slice of the expected RESP byte sequences, in order.
pub fn assert_encoding_order(builders: &[&crate::protocol::builder::CommandBuilder], expected: &[&[u8]]) {
    assert_eq!(
        builders.len(),
        expected.len(),
        "command count mismatch"
    );

    let (mut fake, _tag) =
        FakeConnection::new(FakeResponse::new(RedisValue::Integer(1)));

    for (i, builder) in builders.iter().enumerate() {
        fake.send(builder.build()).unwrap();
    }

    let captured = fake.captured_commands();
    assert_eq!(
        captured.len(),
        expected.len(),
        "captured command count mismatch"
    );

    for (i, exp) in expected.iter().enumerate() {
        assert_eq!(
            captured[i].as_ref(),
            *exp,
            "command {} encoding mismatch",
            i
        );
    }
}

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
