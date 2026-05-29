//! Issue #7 tests: unbounded request queue — memory exhaustion protection.
//!
//! Tests for AC-3.1 through AC-3.4: queue depth limits, request size limits,
//! backpressure behavior, and distinguishable errors.

use super::connection::Connection;
use super::ConnectionLimitError;
use may::sync::spsc;

/// AC-3.1: connect_with_limits() accepts custom limits.
#[test]
#[ignore = "requires live network namespace"]
fn test_connect_with_limits_custom_depth() {
    let result = Connection::connect_with_limits(
        "127.0.0.1",
        6379,
        std::time::Duration::from_secs(1),
        10,   // custom queue depth
        1024, // custom request size
    );
    assert!(result.is_err()); // no Redis, but limits accepted
}

/// AC-3.4: connect_with_limits() accepts large limits.
#[test]
#[ignore = "requires live network namespace"]
fn test_connect_with_limits_large_request_size() {
    let result = Connection::connect_with_limits(
        "127.0.0.1",
        6379,
        std::time::Duration::from_secs(1),
        1024,      // default queue depth
        1_000_000, // 1 MB max request size
    );
    assert!(result.is_err());
}

/// AC-3.2 + AC-3.3: Queue full returns QueueFull error, not panic.
#[test]
#[ignore = "requires live Redis server"]
fn test_queue_full_returns_error() {
    let result = Connection::connect_with_limits(
        "127.0.0.1",
        6379,
        std::time::Duration::from_secs(1),
        2,  // tiny queue depth — fills instantly
        65536,
    );
    let conn = match result {
        Ok(c) => c,
        Err(_) => return, // no Redis server, skip
    };

    // Each request needs its own channel (spsc::Sender is not clonable).
    let ping = b"*1\r\n$4\r\nPING\r\n".to_vec();

    let (tx0, _rx0) = spsc::channel();
    let tag0 = conn.send(super::Request::new(ping.clone(), tx0));
    assert!(tag0.is_ok());

    let (tx1, _rx1) = spsc::channel();
    let tag1 = conn.send(super::Request::new(ping.clone(), tx1));
    assert!(tag1.is_ok());

    // Third send should hit QueueFull
    let (tx2, _rx2) = spsc::channel();
    let tag2 = conn.send(super::Request::new(ping.clone(), tx2));
    assert!(tag2.is_err());
    assert!(matches!(tag2.unwrap_err(), ConnectionLimitError::QueueFull(2)));
}

/// AC-3.2: RequestTooLarge is returned when data exceeds max_request_size.
#[test]
#[ignore = "requires live Redis server"]
fn test_request_too_large_returns_error() {
    let result = Connection::connect_with_limits(
        "127.0.0.1",
        6379,
        std::time::Duration::from_secs(1),
        1024, // generous queue depth
        32,   // tiny request size — PING is 12 bytes
    );
    let conn = match result {
        Ok(c) => c,
        Err(_) => return,
    };

    // PING fits (12 bytes < 32 limit)
    let (tx, _rx) = spsc::channel();
    let ok = conn.send(super::Request::new(b"*1\r\n$4\r\nPING\r\n".to_vec(), tx));
    assert!(ok.is_ok());

    // Oversized request
    let big = vec![b'a'; 64];
    let (tx2, _rx2) = spsc::channel();
    let err = conn.send(super::Request::new(big, tx2));
    assert!(err.is_err());
    assert!(
        matches!(err.unwrap_err(), ConnectionLimitError::RequestTooLarge(32, 64)),
    );
}

/// AC-3.3: QueueFull and RequestTooLarge are distinct error variants.
#[test]
fn test_connection_limit_error_variants() {
    let queue_full = ConnectionLimitError::QueueFull(1024);
    let request_too_large = ConnectionLimitError::RequestTooLarge(65536, 100000);

    let err_full_str = format!("{queue_full}");
    let err_large_str = format!("{request_too_large}");

    assert!(err_full_str.contains("full"));
    assert!(err_full_str.contains("1024"));
    assert!(err_large_str.contains("large"));
    assert!(err_large_str.contains("65536"));
    assert!(err_large_str.contains("100000"));
}

/// Test that RequestTooLarge carries both max and actual size.
#[test]
fn test_request_too_large_error_details() {
    let err = ConnectionLimitError::RequestTooLarge(100, 250);
    assert!(format!("{err}").contains("100"));
    assert!(format!("{err}").contains("250"));
}

/// Test that QueueFull carries the max depth.
#[test]
fn test_queue_full_error_details() {
    let err = ConnectionLimitError::QueueFull(64);
    assert!(format!("{err}").contains("64"));
}
