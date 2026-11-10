#!/usr/bin/env bash
# connect_url credentials against real Redis 7 (tests/url_auth_live.rs): one throwaway container with requirepass
# and an ACL user, one without auth. Nothing else is touched; both are removed on exit.
set -euo pipefail
cd "$(dirname "$0")/.."
AUTH="may-redis-url-auth-$$"
NOAUTH="may-redis-url-noauth-$$"
cleanup() { docker rm -f "$AUTH" "$NOAUTH" > /dev/null 2>&1 || true; }
trap cleanup EXIT
# passwords as tests/url_auth_live.rs encodes them: p+w@d:/?x%y and app+p@ss:1
docker run -d --name "$AUTH" -p 127.0.0.1::6379 redis:7-alpine \
  redis-server --requirepass 'p+w@d:/?x%y' --user app on '>app+p@ss:1' '~*' '&*' '+@all' > /dev/null
docker run -d --name "$NOAUTH" -p 127.0.0.1::6379 redis:7-alpine > /dev/null
AUTH_PORT=$(docker port "$AUTH" 6379/tcp | head -1 | sed 's/.*://')
NOAUTH_PORT=$(docker port "$NOAUTH" 6379/tcp | head -1 | sed 's/.*://')
for c in "$AUTH" "$NOAUTH"; do
  for _ in $(seq 1 50); do docker exec "$c" redis-cli ping 2>/dev/null | grep -q -E 'PONG|NOAUTH' && break; sleep 0.2; done
done
MAY_REDIS_AUTH_PORT="$AUTH_PORT" MAY_REDIS_NOAUTH_PORT="$NOAUTH_PORT" \
  cargo test -j "${JOBS:-3}" --test url_auth_live -- --include-ignored --test-threads 1
echo PASS
