//! Pub/sub push message types and RESP parsing.
//!
//! After `SUBSCRIBE`, Redis sends unsolicited `message` / `pmessage` arrays.
//! Subscribe/unsubscribe acknowledgements are normal request responses.

use crate::core::RedisValue;

/// A pub/sub push or control message from Redis.
///
/// Payloads are binary-safe (`Vec<u8>`): Redis pub/sub payloads are
/// arbitrary bytes, not necessarily UTF-8. Channel and pattern names
/// remain `String`; a push with a non-UTF-8 channel name fails parsing
/// and is escalated to a connection error rather than delivered
/// corrupted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PubSubMessage {
    /// `message` — payload on a subscribed channel.
    Message { channel: String, payload: Vec<u8> },
    /// `pmessage` — payload matching a pattern subscription.
    PatternMessage {
        pattern: String,
        channel: String,
        payload: Vec<u8>,
    },
    /// `subscribe` acknowledgement.
    Subscribe { channel: String, count: i64 },
    /// `unsubscribe` acknowledgement.
    Unsubscribe { channel: String, count: i64 },
    /// `psubscribe` acknowledgement.
    PSubscribe { pattern: String, count: i64 },
    /// `punsubscribe` acknowledgement.
    PUnsubscribe { pattern: String, count: i64 },
}

/// Returns `true` when `value` is an unsolicited pub/sub push (not a command ack).
#[must_use]
pub fn is_pubsub_push(value: &RedisValue) -> bool {
    matches!(
        value,
        RedisValue::Array(items)
            if items.first().and_then(RedisValue::as_str).is_some_and(|k| k == "message" || k == "pmessage")
    )
}

/// Parse a pub/sub push array into [`PubSubMessage`].
#[must_use]
pub fn parse_pubsub_push(value: &RedisValue) -> Option<PubSubMessage> {
    let RedisValue::Array(items) = value else {
        return None;
    };
    let kind = items.first()?.as_str()?;
    match kind {
        "message" if items.len() == 3 => Some(PubSubMessage::Message {
            channel: bulk_to_string(&items[1])?,
            payload: bulk_to_bytes(&items[2])?,
        }),
        "pmessage" if items.len() == 4 => Some(PubSubMessage::PatternMessage {
            pattern: bulk_to_string(&items[1])?,
            channel: bulk_to_string(&items[2])?,
            payload: bulk_to_bytes(&items[3])?,
        }),
        _ => None,
    }
}

/// Parse subscribe/unsubscribe ack arrays (command responses, not pushes).
#[must_use]
pub fn parse_pubsub_ack(value: &RedisValue) -> Option<PubSubMessage> {
    let RedisValue::Array(items) = value else {
        return None;
    };
    let kind = items.first()?.as_str()?;
    match kind {
        "subscribe" if items.len() == 3 => Some(PubSubMessage::Subscribe {
            channel: bulk_to_string(&items[1])?,
            count: items[2].as_integer()?,
        }),
        "unsubscribe" if items.len() == 3 => Some(PubSubMessage::Unsubscribe {
            channel: bulk_to_string(&items[1])?,
            count: items[2].as_integer()?,
        }),
        "psubscribe" if items.len() == 3 => Some(PubSubMessage::PSubscribe {
            pattern: bulk_to_string(&items[1])?,
            count: items[2].as_integer()?,
        }),
        "punsubscribe" if items.len() == 3 => Some(PubSubMessage::PUnsubscribe {
            pattern: bulk_to_string(&items[1])?,
            count: items[2].as_integer()?,
        }),
        _ => None,
    }
}

fn bulk_to_string(value: &RedisValue) -> Option<String> {
    match value {
        RedisValue::BulkString(b) => String::from_utf8(b.clone()).ok(),
        RedisValue::SimpleString(s) => Some(s.clone()),
        _ => None,
    }
}

/// Binary-safe extraction for payloads — never fails on non-UTF-8 bytes.
fn bulk_to_bytes(value: &RedisValue) -> Option<Vec<u8>> {
    match value {
        RedisValue::BulkString(b) => Some(b.clone()),
        RedisValue::SimpleString(s) => Some(s.clone().into_bytes()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    fn bulk(s: &str) -> RedisValue {
        RedisValue::BulkString(s.as_bytes().to_vec())
    }

    #[test]
    fn test_parse_message_push() {
        let value =
            RedisValue::Array(vec![bulk("message"), bulk("news"), bulk("hello")]);
        assert!(is_pubsub_push(&value));
        let msg = parse_pubsub_push(&value).unwrap();
        assert_eq!(
            msg,
            PubSubMessage::Message {
                channel: "news".to_string(),
                payload: b"hello".to_vec(),
            }
        );
    }

    /// Positive: non-UTF-8 payloads are binary-safe — they must parse, not
    /// vanish (previously parse returned None and the push was misrouted
    /// into the command-response queue).
    #[test]
    fn test_parse_message_push_binary_payload() {
        let payload = vec![0xff, 0xfe, 0x00, 0x80];
        let value = RedisValue::Array(vec![
            bulk("message"),
            bulk("news"),
            RedisValue::BulkString(payload.clone()),
        ]);
        let msg = parse_pubsub_push(&value).expect("binary payload must parse");
        assert_eq!(
            msg,
            PubSubMessage::Message {
                channel: "news".to_string(),
                payload,
            }
        );
    }

    /// Positive: binary pmessage payloads parse too.
    #[test]
    fn test_parse_pmessage_push_binary_payload() {
        let payload = vec![0xde, 0xad, 0xbe, 0xef];
        let value = RedisValue::Array(vec![
            bulk("pmessage"),
            bulk("n*"),
            bulk("news"),
            RedisValue::BulkString(payload.clone()),
        ]);
        let msg = parse_pubsub_push(&value).expect("binary payload must parse");
        assert_eq!(
            msg,
            PubSubMessage::PatternMessage {
                pattern: "n*".to_string(),
                channel: "news".to_string(),
                payload,
            }
        );
    }

    /// Negative: a non-UTF-8 *channel name* still fails parsing (and is
    /// escalated to a connection error by decode_responses) — it is never
    /// delivered with a corrupted name.
    #[test]
    fn test_parse_message_push_non_utf8_channel_is_none() {
        let value = RedisValue::Array(vec![
            bulk("message"),
            RedisValue::BulkString(vec![0xff, 0xfe]),
            bulk("hello"),
        ]);
        assert!(parse_pubsub_push(&value).is_none());
    }

    #[test]
    fn test_parse_subscribe_ack_not_push() {
        let value = RedisValue::Array(vec![
            bulk("subscribe"),
            bulk("news"),
            RedisValue::Integer(1),
        ]);
        assert!(!is_pubsub_push(&value));
        let ack = parse_pubsub_ack(&value).unwrap();
        assert_eq!(
            ack,
            PubSubMessage::Subscribe {
                channel: "news".to_string(),
                count: 1,
            }
        );
    }
}
