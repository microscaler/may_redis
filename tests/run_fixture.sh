#!/usr/bin/env bash
set -e

cd /home/casibbald/Workspace/microscaler/may_redis

# Clean up old containers
docker rm -f may-redis-plain-* may-redis-tls-* 2>/dev/null || true

echo "Building fixture..."
cargo test --test fixture_e2e --features test test_fixture_both_containers -- --nocapture --test-threads=1

echo ""
echo "=== Test output ==="
