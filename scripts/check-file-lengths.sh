#!/usr/bin/env bash
# Check that no production Rust source file in src/ exceeds 350 lines.
# Test files (matching *_tests.rs) are excluded from the limit — they have
# looser boundaries because they contain many small self-contained tests.
#
# Exit 0 = all files OK, exit 1 = violations found.
#
# Usage:
#   scripts/check-file-lengths.sh          # default 350
#   scripts/check-file-lengths.sh 200      # custom limit
#
# This is the primary enforcement for the per-file line limit since
# Clippy has no file-level line-count lint.

set -euo pipefail

LIMIT="${1:-350}"
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC_DIR="$REPO_ROOT/src"

EXIT=0

while IFS= read -r -d '' file; do
    # Skip test files (patterns: *_tests.rs and *_tests/*)
    case "$file" in
        *_tests.rs) continue ;;
        *_tests/*) continue ;;
    esac

    count=$(wc -l < "$file")
    if (( count > LIMIT )); then
        echo "FAIL: $(realpath --relative-to="$REPO_ROOT" "$file"): $count lines (limit: $LIMIT)"
        EXIT=1
    fi
done < <(find "$SRC_DIR" -name '*.rs' -type f -print0)

if (( EXIT == 0 )); then
    echo "OK: all production files under $LIMIT lines"
fi

exit $EXIT
