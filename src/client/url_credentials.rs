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

/// Split a URL's authority (after the scheme, before any `?`) into its user information and the host part. The
/// last `@` ends the user information, so an unencoded `@` in a password still parses, though RFC 3986 wants it
/// percent-encoded. An empty user information (`@host`) is none. A `?` in a password must be percent-encoded
/// (`%3F`): unencoded, it starts the query.
pub(super) fn split_userinfo(authority: &str) -> (Option<&str>, &str) {
    authority.rfind('@').map_or((None, authority), |i| {
        (
            Some(&authority[..i]).filter(|u| !u.is_empty()),
            &authority[i + 1..],
        )
    })
}

/// The credentials in a URL's user information: `user:password` (Redis 6 ACL user), `:password` or `password`
/// (the default user). The user is everything before the first `:`, so a password may contain `:`. Both are
/// percent-decoded. An empty user is no user. An empty password with no user (`:@host`) is no credentials; an empty
/// password with a user (`user:@host`) is a parse error rather than an `AUTH` that cannot succeed.
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
    if password.is_empty() {
        return match user {
            Some(_) => Err(RedisError::Parse(
                "URL user information has a user but an empty password".into(),
            )),
            None => Ok((None, None)),
        };
    }
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

    /// The credentials of an authority, as `connect_url` takes them.
    fn creds(authority: &str) -> (Option<String>, Option<String>, String) {
        let (info, host) = split_userinfo(authority);
        let (u, p) = userinfo_credentials(info).unwrap();
        (u, p, host.to_string())
    }

    #[test]
    fn a_url_without_credentials_sends_no_auth() {
        assert_eq!(
            creds("redis.example:6379"),
            (None, None, "redis.example:6379".into())
        );
        assert_eq!(
            creds("@redis.example:6379"),
            (None, None, "redis.example:6379".into())
        );
    }

    #[test]
    fn encoded_reserved_characters_in_the_password_are_decoded() {
        let (u, p, host) = creds("app:a%40b%3Ac%2Fd%3Fe%25f@redis.example:6379");
        assert_eq!(u.as_deref(), Some("app"));
        assert_eq!(p.as_deref(), Some("a@b:c/d?e%f"));
        assert_eq!(host, "redis.example:6379");
    }

    #[test]
    fn a_literal_slash_plus_or_at_in_the_password_still_parses() {
        let (u, p, host) = creds("default:a/b+c@d@redis.example:6379");
        assert_eq!(u.as_deref(), Some("default"));
        assert_eq!(p.as_deref(), Some("a/b+c@d"));
        assert_eq!(host, "redis.example:6379");
    }

    #[test]
    fn an_ipv6_host_keeps_its_brackets_and_port() {
        let (u, p, host) = creds("app:s3cret@[::1]:6379");
        assert_eq!((u.as_deref(), p.as_deref()), (Some("app"), Some("s3cret")));
        assert_eq!(host, "[::1]:6379");
        assert_eq!(creds("[::1]:6379"), (None, None, "[::1]:6379".into()));
        let (u, p, host) = creds(":s3cret@[2001:db8::7]:6380");
        assert_eq!((u, p.as_deref()), (None, Some("s3cret")));
        assert_eq!(host, "[2001:db8::7]:6380");
    }

    #[test]
    fn an_empty_password_is_none_without_a_user_and_an_error_with_one() {
        assert_eq!(userinfo_credentials(Some(":")).unwrap(), (None, None));
        let err = userinfo_credentials(Some("app:")).unwrap_err();
        assert!(err.to_string().contains("empty password"), "{err}");
    }

    #[test]
    fn a_truncated_or_invalid_escape_is_an_error() {
        assert!(userinfo_credentials(Some("app:abc%4")).is_err());
        assert!(userinfo_credentials(Some("app:abc%zz")).is_err());
    }

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
