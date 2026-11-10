//! Credentials in a `redis://` URL's user information ([`super::client_url::connect_url`]): split into user and
//! password, percent-decoded (RFC 3986 §3.2.1), and the `AUTH` command they call for.

use crate::core::RedisError;
use crate::protocol::builder::CommandBuilder;

/// Percent-decode the user information of a URL (RFC 3986 §3.2.1). Unlike [`url_decode`], `+` is kept: it means a
/// space only in query strings, and a password may contain it.
pub(super) fn percent_decode_userinfo(s: &str) -> Result<String, RedisError> {
    let mut result = Vec::with_capacity(s.len());
    let mut bytes = s.bytes();
    while let Some(b) = bytes.next() {
        if b == b'%' {
            let hi = bytes.next().ok_or_else(|| {
                RedisError::Parse("truncated percent-encoding in URL".into())
            })?;
            let lo = bytes.next().ok_or_else(|| {
                RedisError::Parse("truncated percent-encoding in URL".into())
            })?;
            result.push(hex_digit(hi)? * 16 + hex_digit(lo)?);
        } else {
            result.push(b);
        }
    }
    String::from_utf8(result)
        .map_err(|_| RedisError::Parse("url-decoded bytes are not valid UTF-8".into()))
}

/// The credentials in a URL's user information: `user:password` (Redis 6 ACL user), `:password` or `password`
/// (the default user). The user is everything before the first `:`, so a password may contain `:`. Both are
/// percent-decoded. An empty user is no user.
pub(super) fn userinfo_credentials(
    userinfo: Option<&str>,
) -> Result<(Option<String>, Option<String>), RedisError> {
    let Some(info) = userinfo else {
        return Ok((None, None));
    };
    let (user, password) = match info.split_once(':') {
        Some((user, password)) => (Some(user).filter(|u| !u.is_empty()), password),
        None => (None, info),
    };
    Ok((
        user.map(percent_decode_userinfo).transpose()?,
        Some(percent_decode_userinfo(password)?),
    ))
}

/// The `AUTH` for the URL's credentials: `AUTH user password` with a user, `AUTH password` without.
pub(super) fn auth_command(user: Option<&str>, password: &str) -> CommandBuilder {
    let mut cmd = CommandBuilder::new("AUTH");
    if let Some(user) = user {
        cmd = cmd.arg(user);
    }
    cmd.arg(password)
}

fn hex_digit(b: u8) -> Result<u8, RedisError> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        b'A'..=b'F' => Ok(b - b'A' + 10),
        _ => Err(RedisError::Parse("invalid percent-encoding in URL".into())),
    }
}

#[cfg(test)]
mod tests {
    //! Coverage: how the user information becomes credentials, and the AUTH it sends.
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn user_and_password_are_split() {
        let (u, p) = userinfo_credentials(Some("default:s3cret")).unwrap();
        assert_eq!(u.as_deref(), Some("default"));
        assert_eq!(p.as_deref(), Some("s3cret"));
    }

    #[test]
    fn a_password_alone_has_no_user() {
        for info in [":s3cret", "s3cret"] {
            let (u, p) = userinfo_credentials(Some(info)).unwrap();
            assert_eq!(u, None, "{info}");
            assert_eq!(p.as_deref(), Some("s3cret"), "{info}");
        }
        assert_eq!(userinfo_credentials(None).unwrap(), (None, None));
    }

    #[test]
    fn the_password_may_hold_colons_and_encoded_characters_and_keeps_plus() {
        let (u, p) = userinfo_credentials(Some("app:a:b%40c+d")).unwrap();
        assert_eq!(u.as_deref(), Some("app"));
        assert_eq!(p.as_deref(), Some("a:b@c+d"));
    }

    #[test]
    fn auth_sends_the_user_only_when_there_is_one() {
        let with_user = auth_command(Some("app"), "pw").build().unwrap();
        assert_eq!(
            &with_user[..],
            &b"*3\r\n$4\r\nAUTH\r\n$3\r\napp\r\n$2\r\npw\r\n"[..]
        );
        let alone = auth_command(None, "pw").build().unwrap();
        assert_eq!(&alone[..], &b"*2\r\n$4\r\nAUTH\r\n$2\r\npw\r\n"[..]);
    }
}
