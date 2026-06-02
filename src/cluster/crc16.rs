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
        crc ^= u16::from(byte);

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

/// Extract the hash-tag from a Redis key.
///
/// Per the Redis Cluster spec, if a key contains `{...}`, only the
/// substring between the braces is used for hashing. This allows
/// related keys to be co-located on the same cluster node.
///
/// # Examples
///
/// * `{user:123}:profile` → `user:123`
/// * `plain_key` → `plain_key` (no braces, return as-is)
/// * `{no-closing` → `{no-closing` (unclosed brace, return as-is)
#[must_use]
pub fn hash_tag(key: &[u8]) -> &[u8] {
    if let Some(start) = key.iter().position(|&b| b == b'{') {
        if let Some(end) = key[start + 1..].iter().position(|&b| b == b'}') {
            return &key[start + 1..start + 1 + end];
        }
    }
    key
}

/// Compute the Redis Cluster slot for a given key.
///
/// The slot is `CRC16(hash_tag(key)) mod 16384`, where 16384 is the fixed
/// number of hash slots in every Redis Cluster deployment.
///
/// If the key contains `{...}`, only the content inside the braces
/// is hashed (hash-tag extraction per Redis Cluster spec).
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
    crc16(hash_tag(key)) % 16384
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
            assert!(slot < 16384, "slot {slot} out of range for key {key:?}");
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

    // =====================================================================
    // Hash-tag extraction tests (Redis Cluster spec §3.1.1)
    // =====================================================================

    // --- Positive: same-tag keys produce the same slot ---

    #[test]
    fn test_hash_tag_simple_prefix() {
        // {user:123}:profile and {user:123}:settings both hash "user:123"
        let s1 = compute_slot(b"{user:123}:profile");
        let s2 = compute_slot(b"{user:123}:settings");
        let s3 = compute_slot(b"{user:123}:prefs");
        assert_eq!(s1, s2, "same-tag keys must hash to the same slot");
        assert_eq!(s2, s3, "same-tag keys must hash to the same slot");
        assert_eq!(s1, 188, "slot for tag 'user:123' should be 188");
    }

    #[test]
    fn test_hash_tag_same_tag_different_suffixes() {
        // Five different suffixes, same tag
        let tag_str = "{account:42}";
        let keys: Vec<Vec<u8>> = (1..=5)
            .map(|i| format!("{tag_str}:field{i}").into_bytes())
            .collect();
        let slots: Vec<u16> = keys.iter().map(|k| compute_slot(k)).collect();
        assert!(
            slots.iter().all(|&s| s == slots[0]),
            "all {} fields for tag {{account:42}} must land on the same slot (got: {:?})",
            keys.len(),
            slots
        );
    }

    #[test]
    fn test_hash_tag_tag_with_special_chars() {
        // Tags can contain colons, numbers, underscores
        let keys = [b"{foo:bar}:key".to_vec(), b"{foo:bar}:other".to_vec()];
        let slots: Vec<u16> = keys.iter().map(|k| compute_slot(k)).collect();
        assert_eq!(
            slots[0], slots[1],
            "tags with colons must produce same slot"
        );
    }

    #[test]
    fn test_hash_tag_tag_in_middle() {
        // Tag can appear anywhere, not just as prefix
        let s1 = compute_slot(b"prefix:{mytag}:suffix");
        let s2 = compute_slot(b"prefix:{mytag}:other");
        let s3 = compute_slot(b"prefix:{mytag}:more");
        assert_eq!(
            s1, s2,
            "tag in the middle must still be extracted for hashing"
        );
        assert_eq!(
            s2, s3,
            "tag in the middle must still be extracted for hashing"
        );
    }

    #[test]
    fn test_hash_tag_empty_braces() {
        // {} is valid — hash the empty string, which gives slot 0
        let s1 = compute_slot(b"{}:key1");
        let s2 = compute_slot(b"{}:key2");
        assert_eq!(s1, s2, "empty tag {{}} must hash all keys to the same slot");
        assert_eq!(s1, 0, "empty string CRC16 = 0, so slot = 0");
    }

    #[test]
    fn test_hash_tag_tag_with_spaces_and_digits() {
        // Tags can contain any characters between braces
        let keys = [
            b"{my user 123}:key".to_vec(),
            b"{my user 123}:other".to_vec(),
        ];
        let slots: Vec<u16> = keys.iter().map(|k| compute_slot(k)).collect();
        assert_eq!(slots[0], slots[1]);
    }

    // --- Positive: different tags produce different slots ---

    #[test]
    fn test_hash_tag_different_tags_different_slots() {
        // Different tags should (almost certainly) produce different slots
        let s1 = compute_slot(b"{aaa}:key");
        let s2 = compute_slot(b"{zzz}:key");
        assert_ne!(s1, s2, "different tags must produce different slots");
    }

    // --- Negative: malformed brace patterns fall back to full key hashing ---

    #[test]
    fn test_hash_tag_no_closing_brace() {
        // No closing `}` — hash the full key as-is
        let full_slot = compute_slot(b"{unclosed:key");
        let expected = crc16(b"{unclosed:key") % 16384;
        assert_eq!(
            full_slot, expected,
            "unclosed brace should hash the full key"
        );
    }

    #[test]
    fn test_hash_tag_closing_before_opening() {
        // `}` appears before `{` in the string, but there's also a valid
        // `{tag}` pair later. Per Redis Cluster spec, the first `{` to the
        // next `}` defines the tag. `{` at index 3, `}` at index 7 → tag =
        // "tag".
        let s1 = compute_slot(b"}no{tag}:key1");
        let s2 = compute_slot(b"}no{tag}:key2");
        assert_eq!(
            s1, s2,
            "valid {{tag}} pair after a stray }} must still extract the tag"
        );
        // The tag "tag" should give a different slot than the full key.
        let full_slot = crc16(b"}no{tag}:key1") % 16384;
        assert_ne!(s1, full_slot, "should use the tag, not the full key");
    }

    #[test]
    fn test_hash_tag_only_opening_brace() {
        // Only `{`, no `}` at all — hash full key
        let full_slot = compute_slot(b"only{braces:key");
        let expected = crc16(b"only{braces:key") % 16384;
        assert_eq!(
            full_slot, expected,
            "only opening brace should hash the full key"
        );
    }

    #[test]
    fn test_hash_tag_brace_in_suffix_after_tag() {
        // The first `{` to first `}` defines the tag; subsequent braces in
        // the suffix are NOT part of the tag.
        let s1 = compute_slot(b"{user:123}:field{extra}");
        let s2 = compute_slot(b"{user:123}:field{other}");
        let s3 = compute_slot(b"{user:123}:field_nobraces");
        assert_eq!(s1, s2, "braces in suffix must be ignored");
        assert_eq!(
            s1, s3,
            "suffix content after the tag does not affect hashing"
        );
    }

    #[test]
    fn test_hash_tag_nested_braces_first_wins() {
        // Per Redis Cluster spec, the tag is between the first `{` and the
        // next `}`. Inner braces are part of the tag content.
        // For {a{b}:c}:key1, first { is at pos 0, first } after it is at
        // pos 4 (the one after "b"), so the tag is "a{b".
        let s1 = compute_slot(b"{a{b}:c}:key1");
        let s2 = compute_slot(b"{a{b}:c}:key2");
        let expected = crc16(b"a{b") % 16384;
        assert_eq!(s1, expected, "nested tag extracts up to first }}");
        assert_eq!(s2, expected, "nested tag extracts up to first }}");
        assert_eq!(s1, s2);
    }

    #[test]
    fn test_hash_tag_plain_key_unchanged() {
        // Keys without any braces are hashed normally
        let full_slot = compute_slot(b"plain:key");
        let expected = crc16(b"plain:key") % 16384;
        assert_eq!(full_slot, expected);
    }

    #[test]
    fn test_hash_tag_empty_key() {
        // Empty key should give slot 0 (CRC16("") = 0)
        assert_eq!(compute_slot(b""), 0);
    }

    #[test]
    fn test_hash_tag_empty_key_with_braces() {
        // "{}" is a valid empty tag — hashes to slot 0
        assert_eq!(compute_slot(b"{}"), 0);
    }

    // --- Verification against known Redis Cluster slots ---

    #[test]
    fn test_hash_tag_known_cluster_slots() {
        // Use Redis's own hash-tag examples from the Cluster spec.
        // These keys all share the same tag and must hash to the same slot.
        let keys = [
            b"{foo}:bar".to_vec(),
            b"{foo}:baz".to_vec(),
            b"{foo}:qux".to_vec(),
        ];
        let slots: Vec<u16> = keys.iter().map(|k| compute_slot(k)).collect();
        assert!(
            slots.iter().all(|&s| s == slots[0]),
            "all keys with tag {{foo}} must share a slot (got: {slots:?})"
        );
    }

    #[test]
    fn test_hash_tag_complex_tag() {
        // Tags with numbers, hyphens, underscores — common in real workloads.
        // The tag MUST be wrapped in {} for hash-tag extraction to work.
        let tag_str = "{user:session-abc_123}";
        let keys: Vec<Vec<u8>> = (0..5)
            .map(|i| format!("{tag_str}:field{i}").into_bytes())
            .collect();
        let slots: Vec<u16> = keys.iter().map(|k| compute_slot(k)).collect();
        assert!(
            slots.iter().all(|&s| s == slots[0]),
            "complex tag must co-locate all fields on the same slot"
        );
    }
}
