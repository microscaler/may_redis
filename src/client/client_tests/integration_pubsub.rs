#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::unit::{run_integration, shared_client};
use crate::connection::PubSubMessage;
use crate::protocol::commands::{AdminCommands, PubsubCommands};
use crate::PubSubClient;

#[test]
fn test_integration_pubsub_subscribe_receive() {
    run_integration(|| {
        let publisher = shared_client();
        publisher.execute::<()>(publisher.flushdb()).ok();

        let subscriber =
            PubSubClient::connect("127.0.0.1", 6379).expect("pubsub connect");
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
                payload: "hello pubsub".to_string(),
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

        let subscriber =
            PubSubClient::connect("127.0.0.1", 6379).expect("pubsub connect");
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
                payload: "score 3-1".to_string(),
            }
        );

        subscriber.unsubscribe_all().unwrap();
        publisher.execute::<()>(publisher.flushdb()).ok();
    });
}
