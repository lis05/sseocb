#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$SCRIPT_DIR"

CLANG="${CLANG:-clang}"
CLANG_GUEST="${CLANG_GUEST:-$REPO_ROOT/scripts/clang-guest}"
O_TOOL="$REPO_ROOT/o"

MODE_BENCH=0
MODE_NATIVE=0
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
        --debug)
            DEBUG=1
            ;;
        --run|--no-debug)
            DEBUG=0
            ;;
        --clean)
            rm -rf artifacts
            ;;
        --blaris)
            EXTRA_ARGS+=(--decompressor blaris)
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
            echo "  --bench       Run benchmark comparison between native and SSEOCB"
            echo "  --native      Run native target (without SSEOCB)"
            echo "  --debug       Run in interactive simulator debugger (default unless --bench)"
            echo "  --run         Run headless without debugger and print UART output"
            echo "  --blaris      Use blaris decompressor (default: noop)"
            echo "  --noop        Use noop decompressor"
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

mkdir -p artifacts

# 1. Compile main.c as rv32e (guest code)
if [[ $LOG -eq 1 ]]; then
    echo "+ $CLANG_GUEST -I $SCRIPT_DIR -c main.c -o artifacts/guest.o"
fi
"$CLANG_GUEST" \
    -I "$SCRIPT_DIR" \
    -c main.c \
    -o artifacts/guest.o


# 2. Compile native.c as rv32e (native code)
if [[ $LOG -eq 1 ]]; then
    echo "+ $CLANG --target=riscv32 -march=rv32e -mabi=ilp32e -mno-relax -ffreestanding -nostdlib -fno-optimize-sibling-calls -O2 -I $SCRIPT_DIR -c native.c -o artifacts/guest-native.o"
fi
"$CLANG" \
    --target=riscv32 \
    -march=rv32e \
    -mabi=ilp32e \
    -mno-relax \
    -ffreestanding \
    -nostdlib \
    -fno-optimize-sibling-calls \
    -O2 \
    -I "$SCRIPT_DIR" \
    -c native.c \
    -o artifacts/guest-native.o

# 3. Assemble command for orchestrator
if [[ $MODE_BENCH -eq 1 ]]; then
    cmd=(
        "$O_TOOL" run bench
        --guest-obj artifacts/guest.o
        --guest-native-obj artifacts/guest-native.o
        --memory-ld memory.ld
        --output artifacts
        --print-output
    )
elif [[ $MODE_NATIVE -eq 1 ]]; then
    cmd=(
        "$O_TOOL" run native
        --guest-obj artifacts/guest.o
        --guest-native-obj artifacts/guest-native.o
        --memory-ld memory.ld
        --output artifacts
    )
    if [[ $DEBUG -eq 1 ]]; then
        cmd+=(--debug)
    else
        cmd+=(--print-output)
    fi
else
    cmd=(
        "$O_TOOL" run sseocb
        --guest-obj artifacts/guest.o
        --guest-native-obj artifacts/guest-native.o
        --memory-ld memory.ld
        --output artifacts
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
