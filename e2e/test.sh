#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

CLANG="${CLANG:-clang}"
LD_LLD="${LD_LLD:-ld.lld}"
O_TOOL="$REPO_ROOT/o"

COMMON_DIR="$REPO_ROOT/e2e/common"
MEMORY_LD="$COMMON_DIR/memory.ld"
TESTS_DIR="$REPO_ROOT/e2e/tests"

PASS_COUNT=0
FAIL_COUNT=0

for test_dir in "$TESTS_DIR"/*; do
    [[ -d "$test_dir" ]] || continue
    test_name="$(basename "$test_dir")"
    echo "=== Running E2E test: $test_name ==="

    build_dir="$test_dir/build"
    rm -rf "$build_dir"
    mkdir -p "$build_dir"

    # 1. Compile all .c files in test directory as rv32e
    for c_file in "$test_dir"/*.c; do
        [[ -f "$c_file" ]] || continue
        base_name="$(basename "$c_file" .c)"
        "$CLANG" \
            --target=riscv32 \
            -march=rv32e \
            -mabi=ilp32e \
            -mno-relax \
            -ffreestanding \
            -nostdlib \
            -O2 \
            -I "$COMMON_DIR" \
            -I "$test_dir" \
            -c "$c_file" \
            -o "$build_dir/${base_name}.o"
    done

    if [[ ! -f "$build_dir/main.o" ]]; then
        echo "Error: missing main.o in $test_name" >&2
        FAIL_COUNT=$((FAIL_COUNT + 1))
        continue
    fi

    # 2. Combine everything except main.o into native.o
    shopt -s nullglob
    other_objs=("$build_dir"/*.o)
    filtered_objs=()
    for obj in "${other_objs[@]}"; do
        if [[ "$(basename "$obj")" != "main.o" && "$(basename "$obj")" != "native.o" ]]; then
            filtered_objs+=("$obj")
        fi
    done
    shopt -u nullglob

    if [[ ${#filtered_objs[@]} -gt 0 ]]; then
        if [[ -f "$build_dir/native.o" ]]; then
            filtered_objs+=("$build_dir/native.o")
        fi
        "$LD_LLD" -r "${filtered_objs[@]}" -o "$build_dir/native.combined.o"
        mv "$build_dir/native.combined.o" "$build_dir/native.o"
    fi

    # 3. Assemble arguments and run `o`
    cmd=(
        "$O_TOOL" run bench
        --guest-obj "$build_dir/main.o"
        --memory-ld "$MEMORY_LD"
        --output "$build_dir/artifacts"
    )

    if [[ -f "$build_dir/native.o" ]]; then
        cmd+=(--guest-native-obj "$build_dir/native.o")
    fi

    if [[ -f "$test_dir/expected.txt" ]]; then
        cmd+=(--expected-output "$test_dir/expected.txt")
    fi

    if [[ -f "$test_dir/guest.ld" ]]; then
        cmd+=(--guest-ld "$test_dir/guest.ld")
    fi

    if [[ -f "$test_dir/native.ld" ]]; then
        cmd+=(--guest-native-ld "$test_dir/native.ld")
    fi

    if "${cmd[@]}"; then
        echo "PASS: $test_name"
        PASS_COUNT=$((PASS_COUNT + 1))
    else
        echo "FAIL: $test_name" >&2
        FAIL_COUNT=$((FAIL_COUNT + 1))
    fi
    echo ""
done

if [[ $FAIL_COUNT -eq 0 && $PASS_COUNT -gt 0 ]]; then
    echo "All $PASS_COUNT E2E tests passed successfully!"
    exit 0
else
    echo "E2E tests failed: $FAIL_COUNT failed out of $((PASS_COUNT + FAIL_COUNT))." >&2
    exit 1
fi
