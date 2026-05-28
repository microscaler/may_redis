// ToRedisArgs — Convert Rust types into Redis command arguments.
//
// This trait mirrors the `ToRedisArgs` trait from the `redis` crate and
// provides the conversion logic used by the command builder.

/// Trait for types that can be used as arguments to a Redis command.
///
/// Implementations convert a Rust value into raw bytes that are later
/// encoded into RESP bulk strings by the codec layer.
pub trait ToRedisArgs {
    /// Write the serialized representation of this argument into `buf`.
    ///
    /// The bytes written will be encoded as a RESP bulk string (`$N\r\n...`)
    /// by the codec layer.
    fn write_redis_args(&self, buf: &mut Vec<Vec<u8>>);

    /// Return `true` if this argument is a "simple arg" — i.e., it can be
    /// encoded as a single bulk string without any special handling.
    fn is_simple_arg(&self) -> bool;
}

// ---------------------------------------------------------------------------
// Primitive implementations
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Blanket impl: &T delegates to T's implementation
// ---------------------------------------------------------------------------
// If T implements ToRedisArgs, then &T also works by delegating.
// This is the standard Rust pattern and avoids &String not implementing
// ToRedisArgs when only String does.

impl<T: ToRedisArgs> ToRedisArgs for &T {
    fn write_redis_args(&self, buf: &mut Vec<Vec<u8>>) {
        (*self).write_redis_args(buf);
    }

    fn is_simple_arg(&self) -> bool {
        (*self).is_simple_arg()
    }
}

// ---------------------------------------------------------------------------
// Vec<T> delegates to T's implementation
// ---------------------------------------------------------------------------

impl<T: ToRedisArgs> ToRedisArgs for Vec<T> {
    fn write_redis_args(&self, buf: &mut Vec<Vec<u8>>) {
        for item in self {
            item.write_redis_args(buf);
        }
    }

    fn is_simple_arg(&self) -> bool {
        false
    }
}

impl ToRedisArgs for String {
    fn write_redis_args(&self, buf: &mut Vec<Vec<u8>>) {
        buf.push(self.as_bytes().to_vec());
    }

    fn is_simple_arg(&self) -> bool {
        true
    }
}

impl ToRedisArgs for &str {
    fn write_redis_args(&self, buf: &mut Vec<Vec<u8>>) {
        buf.push(self.as_bytes().to_vec());
    }

    fn is_simple_arg(&self) -> bool {
        true
    }
}

impl ToRedisArgs for i64 {
    fn write_redis_args(&self, buf: &mut Vec<Vec<u8>>) {
        buf.push(self.to_string().into_bytes());
    }

    fn is_simple_arg(&self) -> bool {
        true
    }
}

impl ToRedisArgs for u32 {
    fn write_redis_args(&self, buf: &mut Vec<Vec<u8>>) {
        buf.push(self.to_string().into_bytes());
    }

    fn is_simple_arg(&self) -> bool {
        true
    }
}

impl ToRedisArgs for f64 {
    fn write_redis_args(&self, buf: &mut Vec<Vec<u8>>) {
        if self.is_nan() {
            buf.push(b"nan".to_vec());
        } else if self.is_infinite() {
            if self.is_sign_positive() {
                buf.push(b"inf".to_vec());
            } else {
                buf.push(b"-inf".to_vec());
            }
        } else {
            let s = self.to_string();
            // f64::to_string() drops the trailing .0 for whole numbers (e.g., "1" not "1.0")
            // Redis expects the decimal point to always be present.
            if !s.contains('.') && !s.contains('e') && !s.contains('E') {
                buf.push(format!("{s}.0").into_bytes());
            } else {
                buf.push(s.into_bytes());
            }
        }
    }

    fn is_simple_arg(&self) -> bool {
        true
    }
}

impl ToRedisArgs for &[u8] {
    fn write_redis_args(&self, buf: &mut Vec<Vec<u8>>) {
        buf.push(self.to_vec());
    }

    fn is_simple_arg(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_redis_args_string() {
        let mut buf = Vec::new();
        let s = String::from("SET");
        s.write_redis_args(&mut buf);
        assert_eq!(buf, vec![b"SET".to_vec()]);
    }

    #[test]
    fn test_to_redis_args_i64() {
        let mut buf = Vec::new();
        let n: i64 = 42;
        n.write_redis_args(&mut buf);
        assert_eq!(buf, vec![b"42".to_vec()]);
    }

    #[test]
    fn test_to_redis_args_u32() {
        let mut buf = Vec::new();
        let n: u32 = 60;
        n.write_redis_args(&mut buf);
        assert_eq!(buf, vec![b"60".to_vec()]);
    }

    #[test]
    fn test_to_redis_args_str() {
        let mut buf = Vec::new();
        let s: &str = "hello";
        s.write_redis_args(&mut buf);
        assert_eq!(buf, vec![b"hello".to_vec()]);
    }

    #[test]
    fn test_to_redis_args_bytes() {
        let mut buf = Vec::new();
        let data: &[u8] = &[0x01, 0x02, 0x03];
        data.write_redis_args(&mut buf);
        assert_eq!(buf, vec![vec![0x01, 0x02, 0x03]]);
    }

    #[test]
    fn test_to_redis_args_vec_string() {
        let mut buf = Vec::new();
        let v = vec!["A".to_string(), "B".to_string()];
        v.write_redis_args(&mut buf);
        assert_eq!(buf, vec![b"A".to_vec(), b"B".to_vec()]);
    }

    #[test]
    fn test_to_redis_args_is_simple() {
        let s = String::from("test");
        assert!(s.is_simple_arg());

        let v = vec!["a".to_string(), "b".to_string()];
        assert!(!v.is_simple_arg());
    }

    #[test]
    fn test_to_redis_args_empty_vec() {
        let mut buf = Vec::new();
        let v: Vec<String> = vec![];
        v.write_redis_args(&mut buf);
        assert!(buf.is_empty());
    }
}
