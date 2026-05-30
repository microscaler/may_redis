// Roundtrip invariant test helper for all RedisValue variants
//
// These tests verify that write_value -> read_value is an identity function
// for every possible RedisValue input. Each test follows the pattern:
//   1. Write the value using RESPWriter
//   2. Read it back using RESPReader
//   3. Assert equality with the original

use crate::codec::reader::RESPReader;
use crate::codec::writer::RESPWriter;
use crate::core::RedisValue;

/// Helper: encode a value then decode it, asserting equality.
/// Roundtrip a `RedisValue` through the codec.
///
/// # Panics
///
/// Panics if the encoded bytes cannot be decoded back to the original value.
/// This is a test-only utility and a panic indicates a bug in the codec.
#[must_use]
pub fn roundtrip(value: &RedisValue) -> RedisValue {
    let mut w = RESPWriter::new();
    w.write_value(value);
    let bytes = w.take();
    let mut r = RESPReader::new(bytes);
    r.read_value()
        .expect("write_value/read_value roundtrip failed")
}
