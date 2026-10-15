#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

CLANG="${CLANG:-clang}"
CLANG_GUEST="${CLANG_GUEST:-$REPO_ROOT/scripts/clang-guest}"
LD_LLD="${LD_LLD:-ld.lld}"
O_TOOL="$REPO_ROOT/o"

COMMON_DIR="$REPO_ROOT/e2e/common"
MEMORY_LD="$COMMON_DIR/memory.ld"
TESTS_DIR="$REPO_ROOT/e2e/tests"

DECOMPRESSORS=()
SELECTED_TESTS=()
EXTRA_ARGS=()

SHOW_SIZE=1

while [[ $# -gt 0 ]]; do
    case "$1" in
        --decompressor)
            if [[ "$2" == "all" ]]; then
                DECOMPRESSORS=("native" "noop" "graemi" "blaris")
            else
                DECOMPRESSORS+=("$2")
            fi
            shift 2
            ;;
        --decompressor=*)
            val="${1#*=}"
            if [[ "$val" == "all" ]]; then
                DECOMPRESSORS=("native" "noop" "graemi" "blaris")
            else
                DECOMPRESSORS+=("$val")
            fi
            shift 1
            ;;
        native|noop|graemi|blaris)
            DECOMPRESSORS+=("$1")
            shift 1
            ;;
        all)
            DECOMPRESSORS=("native" "noop" "graemi" "blaris")
            shift 1
            ;;
        --test)
            SELECTED_TESTS+=("$2")
            shift 2
            ;;
        --test=*)
            SELECTED_TESTS+=("${1#*=}")
            shift 1
            ;;
        --size)
            SHOW_SIZE=1
            shift 1
            ;;
        --no-size)
            SHOW_SIZE=0
            shift 1
            ;;
        *)
            if [[ -d "$TESTS_DIR/$1" ]]; then
                SELECTED_TESTS+=("$1")
            else
                EXTRA_ARGS+=("$1")
            fi
            shift 1
            ;;
    esac
done

if [[ ${#DECOMPRESSORS[@]} -eq 0 ]]; then
    DECOMPRESSORS=("native" "noop" "graemi" "blaris")
fi

TEST_DIRS=()
if [[ ${#SELECTED_TESTS[@]} -gt 0 ]]; then
    for t in "${SELECTED_TESTS[@]}"; do
        if [[ -d "$TESTS_DIR/$t" ]]; then
            TEST_DIRS+=("$TESTS_DIR/$t")
        else
            echo "Error: test directory not found: $TESTS_DIR/$t" >&2
            exit 1
        fi
    done
else
    for test_dir in "$TESTS_DIR"/*; do
        [[ -d "$test_dir" ]] || continue
        TEST_DIRS+=("$test_dir")
    done
fi

TOTAL_PASS=0
TOTAL_FAIL=0
FAILED_RUNS=()

for decompressor in "${DECOMPRESSORS[@]}"; do
    if [[ ${#DECOMPRESSORS[@]} -gt 1 ]]; then
        echo "=================================================="
        echo "Running E2E tests with compressor: $decompressor"
        echo "=================================================="
    else
        echo "Running E2E tests with compressor: $decompressor"
    fi

    PASS_COUNT=0
    FAIL_COUNT=0

    for test_dir in "${TEST_DIRS[@]}"; do
        test_name="$(basename "$test_dir")"
        if [[ ${#DECOMPRESSORS[@]} -gt 1 ]]; then
            echo "=== Running E2E test: $test_name [$decompressor] ==="
        else
            echo "=== Running E2E test: $test_name ==="
        fi

        build_dir="$test_dir/build"
        rm -rf "$build_dir"
        mkdir -p "$build_dir"

        # 1. Compile all .c files in test directory as rv32e
        for c_file in "$test_dir"/*.c; do
            [[ -f "$c_file" ]] || continue
            base_name="$(basename "$c_file" .c)"
            if [[ "$base_name" == "main" ]]; then
                "$CLANG_GUEST" \
                    -I "$COMMON_DIR" \
                    -I "$test_dir" \
                    -c "$c_file" \
                    -o "$build_dir/${base_name}.o"
            else
                "$CLANG" \
                    --target=riscv32 \
                    -march=rv32e \
                    -mabi=ilp32e \
                    -mno-relax \
                    -ffreestanding \
                    -nostdlib \
                    -fno-optimize-sibling-calls \
                    -O2 \
                    -I "$COMMON_DIR" \
                    -I "$test_dir" \
                    -c "$c_file" \
                    -o "$build_dir/${base_name}.o"
            fi
        done

        if [[ ! -f "$build_dir/main.o" ]]; then
            echo "Error: missing main.o in $test_name" >&2
            FAIL_COUNT=$((FAIL_COUNT + 1))
            TOTAL_FAIL=$((TOTAL_FAIL + 1))
            FAILED_RUNS+=("$test_name [$decompressor]")
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
        if [[ "$decompressor" == "native" ]]; then
            cmd=(
                "$O_TOOL" run native
                --guest-obj "$build_dir/main.o"
                --memory-ld "$MEMORY_LD"
                --output "$build_dir/artifacts"
                --print-output
            )
        else
            cmd=(
                "$O_TOOL" run bench
                --guest-obj "$build_dir/main.o"
                --memory-ld "$MEMORY_LD"
                --output "$build_dir/artifacts"
                --decompressor "$decompressor"
                --print-output
            )
        fi

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

        if [[ -f "$test_dir/flags.txt" ]]; then
            while IFS= read -r line || [[ -n "$line" ]]; do
                [[ -z "$line" || "$line" =~ ^# ]] && continue
                read -ra words <<< "$line"
                cmd+=("${words[@]}")
            done < "$test_dir/flags.txt"
        fi

        if [[ ${#EXTRA_ARGS[@]} -gt 0 ]]; then
            cmd+=("${EXTRA_ARGS[@]}")
        fi

        if [[ "$SHOW_SIZE" -eq 1 ]]; then
            cmd+=(--size)
        fi

        if "${cmd[@]}"; then
            if [[ ${#DECOMPRESSORS[@]} -gt 1 ]]; then
                echo "PASS: $test_name [$decompressor]"
            else
                echo "PASS: $test_name"
            fi
            PASS_COUNT=$((PASS_COUNT + 1))
            TOTAL_PASS=$((TOTAL_PASS + 1))
        else
            if [[ ${#DECOMPRESSORS[@]} -gt 1 ]]; then
                echo "FAIL: $test_name [$decompressor]" >&2
            else
                echo "FAIL: $test_name" >&2
            fi
            FAIL_COUNT=$((FAIL_COUNT + 1))
            TOTAL_FAIL=$((TOTAL_FAIL + 1))
            FAILED_RUNS+=("$test_name [$decompressor]")
        fi
        echo ""
    done

    if [[ ${#DECOMPRESSORS[@]} -gt 1 ]]; then
        echo "Summary for compressor $decompressor: $PASS_COUNT passed, $FAIL_COUNT failed."
        echo ""
    fi
done

if [[ $TOTAL_FAIL -eq 0 && $TOTAL_PASS -gt 0 ]]; then
    echo "All $TOTAL_PASS E2E tests passed successfully!"
    exit 0
else
    echo "E2E tests failed: $TOTAL_FAIL failed out of $((TOTAL_PASS + TOTAL_FAIL))." >&2
    if [[ ${#FAILED_RUNS[@]} -gt 0 ]]; then
        echo "Failed tests:" >&2
        for failed in "${FAILED_RUNS[@]}"; do
            echo "  - $failed" >&2
        done
    fi
    exit 1
fi
