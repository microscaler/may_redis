//! `connect_url` credentials against a real Redis 7: `AUTH password` for the default user, `AUTH user password` for
//! an ACL user, passwords holding reserved characters, and the failures a caller must see cleanly.
//!
//! Strategy: `scripts/test-url-auth.sh` starts two throwaway `redis:7` containers, one with `requirepass` and an ACL
//! user, one without auth, and runs these ignored tests with their ports in `MAY_REDIS_AUTH_PORT` and
//! `MAY_REDIS_NOAUTH_PORT`. Each case runs on a may coroutine, as clients do.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use may_redis::{CommandBuilder, RedisClient};
use std::time::{Duration, Instant};

/// The default user's password, as the script sets it: every character a URL reserves, plus `+` and `%`.
pub const DEFAULT_PASS_ENC: &str = "p+w%40d%3A%2F%3Fx%25y";
/// The ACL user's password, as the script sets it.
pub const APP_PASS_ENC: &str = "app%2Bp%40ss%3A1";

fn port(var: &str) -> u16 {
    std::env::var(var)
        .unwrap_or_else(|_| panic!("{var}: run scripts/test-url-auth.sh"))
        .parse()
        .unwrap()
}

fn on_coroutine<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    // SAFETY: the coroutine touches no thread-local state and is joined here.
    let h = unsafe { may::coroutine::Builder::new().stack_size(0x8_0000).spawn(f) }
        .unwrap();
    match h.join() {
        Ok(v) => v,
        Err(p) => std::panic::resume_unwind(p),
    }
}

fn whoami(url: &str) -> Result<String, String> {
    let c = RedisClient::connect_url(url).map_err(|e| e.to_string())?;
    c.execute::<String>(CommandBuilder::new("ACL").arg("WHOAMI"))
        .map_err(|e| e.to_string())
}

#[test]
#[ignore = "needs Redis: scripts/test-url-auth.sh"]
fn the_default_user_authenticates_with_the_password_alone() {
    let p = port("MAY_REDIS_AUTH_PORT");
    on_coroutine(move || {
        for url in [
            format!("redis://:{DEFAULT_PASS_ENC}@127.0.0.1:{p}"),
            format!("redis://{DEFAULT_PASS_ENC}@127.0.0.1:{p}"),
            format!("redis://default:{DEFAULT_PASS_ENC}@127.0.0.1:{p}"),
        ] {
            assert_eq!(whoami(&url).as_deref(), Ok("default"), "{url}");
        }
    });
}

#[test]
#[ignore = "needs Redis: scripts/test-url-auth.sh"]
fn an_acl_user_authenticates_with_user_and_password() {
    let p = port("MAY_REDIS_AUTH_PORT");
    on_coroutine(move || {
        let url = format!("redis://app:{APP_PASS_ENC}@127.0.0.1:{p}");
        assert_eq!(whoami(&url).as_deref(), Ok("app"));
    });
}

#[test]
#[ignore = "needs Redis: scripts/test-url-auth.sh"]
fn a_wrong_password_is_a_clean_auth_error_at_once() {
    let p = port("MAY_REDIS_AUTH_PORT");
    on_coroutine(move || {
        for url in [
            format!("redis://:wrong@127.0.0.1:{p}"),
            format!("redis://app:wrong@127.0.0.1:{p}"),
            format!("redis://nobody:{APP_PASS_ENC}@127.0.0.1:{p}"),
        ] {
            let started = Instant::now();
            let err = RedisClient::connect_url(&url).err().expect("refused");
            let text = err.to_string();
            assert!(
                text.contains("AUTH failed") && text.contains("WRONGPASS"),
                "{url}: {text}"
            );
            assert!(
                started.elapsed() < Duration::from_secs(2),
                "no retry loop: {:?}",
                started.elapsed()
            );
        }
    });
}

#[test]
#[ignore = "needs Redis: scripts/test-url-auth.sh"]
fn no_credentials_on_a_server_that_needs_them_fails_on_the_first_command() {
    let p = port("MAY_REDIS_AUTH_PORT");
    on_coroutine(move || {
        let err = whoami(&format!("redis://127.0.0.1:{p}")).unwrap_err();
        assert!(err.contains("NOAUTH"), "{err}");
    });
}

#[test]
#[ignore = "needs Redis: scripts/test-url-auth.sh"]
fn credentials_on_a_server_without_auth() {
    let p = port("MAY_REDIS_NOAUTH_PORT");
    on_coroutine(move || {
        // no credentials: the default user
        assert_eq!(
            whoami(&format!("redis://127.0.0.1:{p}")).as_deref(),
            Ok("default")
        );
        // the default user has no password there (nopass), so AUTH default <anything> is accepted
        assert_eq!(
            whoami(&format!("redis://default:anything@127.0.0.1:{p}")).as_deref(),
            Ok("default")
        );
        // a password alone is refused by the server, and that answer reaches the caller as an error
        let err = whoami(&format!("redis://:anything@127.0.0.1:{p}")).unwrap_err();
        assert!(err.contains("AUTH failed"), "{err}");
        // a user the server does not have
        let err = whoami(&format!("redis://app:anything@127.0.0.1:{p}")).unwrap_err();
        assert!(err.contains("WRONGPASS"), "{err}");
    });
}

#[test]
fn malformed_urls_are_parse_errors_before_any_connection() {
    // port 9 (discard) is never dialled: each of these fails while parsing
    for url in [
        "redis://app:abc%zz@127.0.0.1:9",
        "redis://app:abc%4@127.0.0.1:9",
        "redis://app:@127.0.0.1:9",
        "http://127.0.0.1:9",
        "redis+sentinel://127.0.0.1:9",
    ] {
        let started = Instant::now();
        let err = RedisClient::connect_url(url).err().expect("a parse error");
        assert!(
            matches!(err, may_redis::RedisError::Parse(_)),
            "{url}: {err}"
        );
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "{url}: no connection attempt"
        );
    }
}
