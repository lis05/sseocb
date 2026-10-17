#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
cd "$SCRIPT_DIR"

O_TOOL="$REPO_ROOT/o"

MODE_BENCH=0
MODE_NATIVE=0
MODE_TEST=0
DEBUG=1
LOG=0
EXTRA_ARGS=()

for arg in "$@"; do
    case "$arg" in
        --bench)
            MODE_BENCH=1
            DEBUG=0
            ;;
        --native)
            MODE_NATIVE=1
            ;;
        --test)
            MODE_TEST=1
            DEBUG=0
            ;;
        --debug)
            DEBUG=1
            ;;
        --run|--no-debug)
            DEBUG=0
            ;;
        --clean)
            make -s clean
            ;;
        --blaris)
            EXTRA_ARGS+=(--decompressor blaris)
            ;;
        --graemi)
            EXTRA_ARGS+=(--decompressor graemi)
            ;;
        --noop)
            EXTRA_ARGS+=(--decompressor noop)
            ;;
        --size)
            EXTRA_ARGS+=(--size)
            ;;
        --log)
            EXTRA_ARGS+=(--log)
            LOG=1
            ;;
        -h|--help)
            echo "Usage: ./all.sh [OPTIONS] [-- EXTRA_O_FLAGS...]"
            echo ""
            echo "Options:"
            echo "  --test        Run headless, compare output with expected_output.txt, print TEST: PASSED/FAILED"
            echo "  --bench       Run benchmark comparison between native and SSEOCB"
            echo "  --native      Run native target (without SSEOCB)"
            echo "  --debug       Run in interactive simulator debugger (default)"
            echo "  --run         Run headless without debugger and print UART output"
            echo "  --blaris      Use blaris decompressor"
            echo "  --graemi      Use graemi decompressor"
            echo "  --noop        Use noop decompressor (default)"
            echo "  --size        Print detailed memory size breakdown"
            echo "  --log         Print every command executed"
            echo "  --clean       Clean artifacts/ directory before building"
            echo "  -h, --help    Show this help message"
            exit 0
            ;;
        *)
            EXTRA_ARGS+=("$arg")
            ;;
    esac
done

# 1. Build guest.o (and guest-native.o if present) via Makefile
if [[ $LOG -eq 1 ]]; then
    make all
else
    make -s all
fi

COMMON_O_ARGS=(
    --guest-obj artifacts/guest.o
    --memory-ld memory.ld
    --output artifacts
)

if [[ -f artifacts/guest-native.o ]]; then
    COMMON_O_ARGS+=(--guest-native-obj artifacts/guest-native.o)
fi

if [[ -f guest.ld ]]; then
    COMMON_O_ARGS+=(--guest-ld guest.ld)
fi

if [[ -f guest-native.ld ]]; then
    COMMON_O_ARGS+=(--guest-native-ld guest-native.ld)
fi

# 2. Assemble command for orchestrator
if [[ $MODE_TEST -eq 1 ]]; then
    if [[ $MODE_NATIVE -eq 1 ]]; then
        cmd=("$O_TOOL" run native "${COMMON_O_ARGS[@]}" --print-output --expected-output expected_output.txt)
    else
        cmd=("$O_TOOL" run sseocb "${COMMON_O_ARGS[@]}" --print-output --expected-output expected_output.txt)
    fi
    if [[ ${#EXTRA_ARGS[@]} -gt 0 ]]; then
        cmd+=("${EXTRA_ARGS[@]}")
    fi

    if "${cmd[@]}"; then
        if ! grep -q "__sseocb_g_plt_fib:" artifacts/trampolines.s; then
            echo "TEST: FAILED (no trampoline)"
            exit 1
        fi
        echo "TEST: PASSED"
        exit 0
    else
        echo "TEST: FAILED"
        exit 1
    fi
elif [[ $MODE_BENCH -eq 1 ]]; then
    cmd=(
        "$O_TOOL" run bench
        "${COMMON_O_ARGS[@]}"
        --print-output
    )
elif [[ $MODE_NATIVE -eq 1 ]]; then
    cmd=(
        "$O_TOOL" run native
        "${COMMON_O_ARGS[@]}"
    )
    if [[ $DEBUG -eq 1 ]]; then
        cmd+=(--debug)
    else
        cmd+=(--print-output)
    fi
else
    cmd=(
        "$O_TOOL" run sseocb
        "${COMMON_O_ARGS[@]}"
    )
    if [[ $DEBUG -eq 1 ]]; then
        cmd+=(--debug)
    else
        cmd+=(--print-output)
    fi
fi

if [[ ${#EXTRA_ARGS[@]} -gt 0 ]]; then
    cmd+=("${EXTRA_ARGS[@]}")
fi

exec "${cmd[@]}"
