#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TESTS_DIR="$SCRIPT_DIR/tests"

DECOMPRESSORS=(--graemi --noop)

for test_dir in "$TESTS_DIR"/*; do
    [[ -d "$test_dir" ]] || continue
    test_name="$(basename "$test_dir")"
    (
        cd "$test_dir"
        for dec in "${DECOMPRESSORS[@]}"; do
            echo "=== [$test_name] $dec ==="
            ./all.sh --test "$dec" "$@"
            echo ""
        done
    )
done

echo "ALL TESTS PASSED"
