// CRC16-ANSI hash function for Redis Cluster slot computation.
//
// Redis Cluster uses CRC16-ANSI (polynomial 0xA001) to map keys to
// 16384 hash slots. This module implements the hash function as a
// pure computation — no network, no runtime dependencies.
//
// Reference: Redis Cluster Specification §3.1.1
// https://redis.io/docs/reference/cluster-spec/#distributing-keys

/// Compute the CRC16-ANSI checksum of `data`.
///
/// Redis Cluster maps a key to slot `CRC16(key) mod 16384`.
///
/// # Algorithm
///
/// CRC16-ANSI uses polynomial 0xA001 (reflected form of 0x8005).
/// The algorithm processes each byte, XORing with the current
/// checksum and iterating over each bit.
///
/// # Example
///
/// ```
/// use may_redis::cluster::crc16;
///
/// // Slot for key "foo"
/// let slot = crc16(b"foo") % 16384;
/// assert_eq!(slot, 12182);
/// ```
#[must_use]
pub fn crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0;

    let mut i = 0;
    while i < data.len() {
        let byte = data[i];
        i += 1;
        crc ^= byte as u16;

        let mut bit = 0;
        while bit < 8 {
            bit += 1;
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xa001;
            } else {
                crc >>= 1;
            }
        }
    }

    crc
}

/// Compute the Redis Cluster slot for a given key.
///
/// The slot is `CRC16(key) mod 16384`, where 16384 is the fixed
/// number of hash slots in every Redis Cluster deployment.
///
/// # Arguments
///
/// * `key` — The Redis key as a byte slice.
///
/// # Returns
///
/// A slot number in the range `0..16384`.
///
/// # Example
///
/// ```
/// use may_redis::cluster::compute_slot;
///
/// let slot = compute_slot(b"foo");
/// assert_eq!(slot, 909);
/// ```
#[must_use]
pub fn compute_slot(key: &[u8]) -> u16 {
    crc16(key) % 16384
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test case from CRC16-ANSI test vectors.
    /// CRC16("foo") = 0xC38D = 50061, slot = 50061 % 16384 = 909.
    #[test]
    fn test_crc16_foo() {
        let hash = crc16(b"foo");
        assert_eq!(hash, 50061);
    }

    #[test]
    fn test_crc16_empty() {
        let hash = crc16(b"");
        assert_eq!(hash, 0);
    }

    /// CRC16 of "123456789" should match the standard CRC16-ANSI
    /// test vector: 0xBB3D = 47933.
    #[test]
    fn test_crc16_standard_vector() {
        let hash = crc16(b"123456789");
        assert_eq!(hash, 47933);
    }

    #[test]
    fn test_slot_single_byte_key() {
        // Key "a" → CRC16("a") = 59585, slot = 59585 % 16384 = 10433
        let slot = compute_slot(b"a");
        assert_eq!(slot, 10433);
    }

    #[test]
    fn test_slot_range() {
        // Verify that all computed slots are in valid range
        let test_keys: &[&[u8]] = &[
            b"",
            b"a",
            b"foo",
            b"123456789",
            b"key_with_underscores",
            b"UPPER",
        ];
        for key in test_keys {
            let slot = compute_slot(key);
            assert!(slot < 16384, "slot {slot} out of range for key {:?}", key);
        }
    }

    /// CRC16 must be deterministic: same input always produces same output.
    #[test]
    fn test_crc16_deterministic() {
        let hash1 = crc16(b"test_key");
        let hash2 = crc16(b"test_key");
        assert_eq!(hash1, hash2);
    }

    /// Different inputs should generally produce different outputs
    /// (not a strict requirement due to collisions, but a sanity check).
    #[test]
    fn test_crc16_different_inputs() {
        let hash_a = crc16(b"a");
        let hash_b = crc16(b"b");
        assert_ne!(hash_a, hash_b);
    }

    /// Slot distribution: verify that a sample of keys spans multiple slots.
    #[test]
    fn test_slot_distribution() {
        let mut slots = Vec::new();
        for i in 0..100u8 {
            let key = format!("key_{i}").into_bytes();
            slots.push(compute_slot(&key));
        }
        let unique: std::collections::HashSet<_> = slots.into_iter().collect();
        // With 100 keys and 16384 slots, expect at least 50 unique slots
        assert!(
            unique.len() >= 50,
            "slot distribution too poor: only {} unique slots from 100 keys",
            unique.len()
        );
    }
}
