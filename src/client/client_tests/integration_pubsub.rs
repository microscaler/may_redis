#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::unit::{integration_redis_port, run_integration, shared_client};
use crate::connection::PubSubMessage;
use crate::protocol::commands::{AdminCommands, PubsubCommands};
use crate::PubSubClient;

#[test]
fn test_integration_pubsub_subscribe_receive() {
    run_integration(|| {
        let publisher = shared_client();
        publisher.execute::<()>(publisher.flushdb()).ok();

        let port = integration_redis_port();
        let subscriber =
            PubSubClient::connect("127.0.0.1", port).expect("pubsub connect");
        may::coroutine::yield_now();
        subscriber.subscribe(&["chan_a"]).unwrap();

        let count: i64 = publisher
            .execute(publisher.publish("chan_a", "hello pubsub"))
            .unwrap();
        assert_eq!(count, 1);

        let msg = subscriber.recv_message().unwrap();
        assert_eq!(
            msg,
            PubSubMessage::Message {
                channel: "chan_a".to_string(),
                payload: b"hello pubsub".to_vec(),
            }
        );

        subscriber.unsubscribe_all().unwrap();
        publisher.execute::<()>(publisher.flushdb()).ok();
    });
}

/// Positive: a published non-UTF-8 payload arrives intact at the subscriber
/// (previously the push failed UTF-8 parsing and was silently misrouted).
#[test]
fn test_integration_pubsub_binary_payload() {
    run_integration(|| {
        let publisher = shared_client();
        publisher.execute::<()>(publisher.flushdb()).ok();

        let port = integration_redis_port();
        let subscriber =
            PubSubClient::connect("127.0.0.1", port).expect("pubsub connect");
        may::coroutine::yield_now();
        subscriber.subscribe(&["chan_bin"]).unwrap();

        let binary: &[u8] = &[0xff, 0xfe, 0x00, 0x80, 0x01];
        let count: i64 = publisher
            .execute(publisher.publish("chan_bin", binary))
            .unwrap();
        assert_eq!(count, 1);

        let msg = subscriber.recv_message().unwrap();
        assert_eq!(
            msg,
            PubSubMessage::Message {
                channel: "chan_bin".to_string(),
                payload: binary.to_vec(),
            }
        );

        subscriber.unsubscribe_all().unwrap();
        publisher.execute::<()>(publisher.flushdb()).ok();
    });
}

#[test]
fn test_integration_pubsub_psubscribe() {
    run_integration(|| {
        let publisher = shared_client();
        publisher.execute::<()>(publisher.flushdb()).ok();

        let port = integration_redis_port();
        let subscriber =
            PubSubClient::connect("127.0.0.1", port).expect("pubsub connect");
        may::coroutine::yield_now();
        subscriber.psubscribe(&["news.*"]).unwrap();

        let count: i64 = publisher
            .execute(publisher.publish("news.sports", "score 3-1"))
            .unwrap();
        assert_eq!(count, 1);

        let msg = subscriber.recv_message().unwrap();
        assert_eq!(
            msg,
            PubSubMessage::PatternMessage {
                pattern: "news.*".to_string(),
                channel: "news.sports".to_string(),
                payload: b"score 3-1".to_vec(),
            }
        );

        subscriber.unsubscribe_all().unwrap();
        publisher.execute::<()>(publisher.flushdb()).ok();
    });
}
