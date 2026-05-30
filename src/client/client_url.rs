// URL decoding helper for redis:// connection URLs.
//
// Handles percent-encoding per RFC 3986, supporting passwords with
// special characters (@, :, /, ?, #, [, ], %).

use crate::core::RedisError;

/// URL-decode a percent-encoded string.
///
/// Only valid `%HH` sequences are decoded; all other characters pass through
/// unchanged. Invalid percent-encoding (e.g. `%GG`) returns a `Parse` error.
/// O(n) with no backtracking.
pub fn url_decode(s: &str) -> Result<String, RedisError> {
    let mut result = String::new();
    let mut chars = s.chars();

    while let Some(ch) = chars.next() {
        if ch == '%' {
            let hi = chars.next().ok_or_else(|| {
                RedisError::Parse("incomplete percent-encoding at end of string".into())
            })?;
            let lo = chars.next().ok_or_else(|| {
                RedisError::Parse("incomplete percent-encoding (missing second hex digit)".into())
            })?;

            let byte = u8::from_str_radix(&format!("{hi}{lo}"), 16).map_err(|_| {
                RedisError::Parse(format!(
                    "invalid percent-encoding %{hi}{lo} (not valid hex)"
                ))
            })?;

            result.push(byte as char);
        } else {
            result.push(ch);
        }
    }

    Ok(result)
}
