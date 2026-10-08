#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Ensure simulator binary is built
cargo build --bin sim --manifest-path "$REPO_ROOT/Cargo.toml" --quiet
SIM_BIN="$REPO_ROOT/target/debug/sim"
PRODUCE_BIN="$REPO_ROOT/scripts/rv32i_produce_bin.sh"

TEST_DIR="$REPO_ROOT/crates/sim/test_run"
ARTIFACTS_DIR="$TEST_DIR/artifacts"
rm -rf "$ARTIFACTS_DIR"
mkdir -p "$ARTIFACTS_DIR"

# Export memory topology script to artifacts/memory.ld for inspection
"$SIM_BIN" export-script > "$ARTIFACTS_DIR/memory.ld"

PASS_COUNT=0
FAIL_COUNT=0

for test_c in "$TEST_DIR"/*.c; do
    [[ -f "$test_c" ]] || continue
    test_name="$(basename "$test_c" .c)"
    bin_file="$ARTIFACTS_DIR/${test_name}.bin"
    expected_out="$TEST_DIR/${test_name}.out"
    actual_out="$ARTIFACTS_DIR/${test_name}.actual"
    err_file="$ARTIFACTS_DIR/${test_name}.err"

    if [[ ! -f "$expected_out" ]]; then
        echo "FAIL: $test_name (missing expected output file: ${test_name}.out)" >&2
        FAIL_COUNT=$((FAIL_COUNT + 1))
        continue
    fi

    # 1. Compile C file to flat RV32I binary using rv32i_produce_bin.sh
    if ! "$PRODUCE_BIN" "$test_c" "$bin_file" > "$err_file" 2>&1; then
        echo "FAIL: $test_name (compilation failed)" >&2
        cat "$err_file" >&2
        FAIL_COUNT=$((FAIL_COUNT + 1))
        continue
    fi

    # 2. Run flat binary through `sim run <file.bin>`
    if ! "$SIM_BIN" run "$bin_file" > "$actual_out" 2> "$err_file"; then
        echo "FAIL: $test_name (simulation failed)" >&2
        cat "$err_file" >&2
        FAIL_COUNT=$((FAIL_COUNT + 1))
        continue
    fi

    # 3. Verify stdout against expected output
    if diff -u "$expected_out" "$actual_out" > /dev/null; then
        echo "PASS: $test_name"
        PASS_COUNT=$((PASS_COUNT + 1))
    else
        echo "FAIL: $test_name (output mismatch)" >&2
        diff -u "$expected_out" "$actual_out" || true
        FAIL_COUNT=$((FAIL_COUNT + 1))
    fi
done

echo ""
if [[ $FAIL_COUNT -eq 0 && $PASS_COUNT -gt 0 ]]; then
    echo "All $PASS_COUNT bare-metal C E2E tests passed!"
    exit 0
else
    echo "$FAIL_COUNT test(s) failed out of $((PASS_COUNT + FAIL_COUNT))." >&2
    echo "Artifacts preserved in '$ARTIFACTS_DIR' for debugging." >&2
    exit 1
fi
