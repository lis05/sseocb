#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 ]]; then
    echo "Usage: $0 <source.c> [output.bin]" >&2
    exit 1
fi

SRC_FILE="$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"
if [[ ! -f "$SRC_FILE" ]]; then
    echo "Error: source file '$1' not found" >&2
    exit 1
fi

if [[ $# -ge 2 ]]; then
    OUT_BIN="$2"
else
    OUT_BIN="${SRC_FILE%.c}.bin"
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
SETUP_DIR="$REPO_ROOT/crates/sim/setup"
ENTRY_S="$SETUP_DIR/entry.s"
LINK_LD="$SETUP_DIR/link.ld"

if [[ ! -f "$ENTRY_S" || ! -f "$LINK_LD" ]]; then
    echo "Error: setup files not found in '$SETUP_DIR'" >&2
    exit 1
fi

# Locate compiler and tools
CLANG="${CLANG:-$(command -v clang || true)}"
if [[ -z "$CLANG" ]]; then
    echo "Error: 'clang' not found in PATH" >&2
    exit 1
fi

OBJCOPY="${OBJCOPY:-$(command -v llvm-objcopy || true)}"
if [[ -z "$OBJCOPY" ]]; then
    echo "Error: 'llvm-objcopy' not found in PATH" >&2
    exit 1
fi

LD_LLD="${LD_LLD:-$(command -v ld.lld || true)}"
if [[ -z "$LD_LLD" ]]; then
    RUSTUP_DIR="${RUSTUP_HOME:-$HOME/.rustup}"
    if [[ -d "$RUSTUP_DIR" ]]; then
        LD_LLD="$(find "$RUSTUP_DIR" -name "ld.lld" -type f -perm -111 2>/dev/null | head -n 1 || true)"
    fi
fi

if [[ -z "$LD_LLD" || ! -x "$LD_LLD" ]]; then
    echo "Error: 'ld.lld' linker not found in PATH or ~/.rustup" >&2
    exit 1
fi

# Ensure simulator is built so we can export the memory layout
SIM_BIN="$REPO_ROOT/target/debug/sim"
if [[ ! -x "$SIM_BIN" ]]; then
    cargo build --bin sim --manifest-path "$REPO_ROOT/Cargo.toml" --quiet
fi

TEMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TEMP_DIR"' EXIT

# Generate memory.ld from the simulator's memory topology
"$SIM_BIN" export-script > "$TEMP_DIR/memory.ld"

mkdir -p "$(dirname "$OUT_BIN")"

# Compile and link into ELF
"$CLANG" \
    --target=riscv32 \
    -march=rv32i \
    -mabi=ilp32 \
    -mno-relax \
    -nostdlib \
    -ffreestanding \
    -O2 \
    -DUART_BASE=0x40000000 \
    -I "$SETUP_DIR" \
    -I "$(dirname "$SRC_FILE")" \
    -I "$TEMP_DIR" \
    --ld-path="$LD_LLD" \
    -Wl,-L,"$TEMP_DIR" \
    -Wl,-T,"$LINK_LD" \
    -o "$TEMP_DIR/prog.elf" \
    "$ENTRY_S" \
    "$SRC_FILE"

# Extract flat binary
"$OBJCOPY" -O binary "$TEMP_DIR/prog.elf" "$OUT_BIN"
