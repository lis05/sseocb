set -e


# Build everything expect interpreter
cargo build -p tool
cargo build -p sim
cargo build -p rv32-isa --release
ln -sf ../target/debug/sim sim
ln -sf ../target/debug/tool tool

SIM_FLASH_BASE=1M
SIM_FLASH_LEN=32K
SIM_SRAM_BASE=2M
SIM_SRAM_LEN=16K
SIM_STACK_BASE=3M
SIM_STACK_LEN=8K
SIM_UART_BASE=0x40000000
SIM_MAX_CYCLES=100M

sim() {
    local cmd="$1"
    shift
    ./sim "$cmd" \
        --flash-base "$SIM_FLASH_BASE" \
        --flash-len "$SIM_FLASH_LEN" \
        --sram-base "$SIM_SRAM_BASE" \
        --sram-len "$SIM_SRAM_LEN" \
        --stack-base "$SIM_STACK_BASE" \
        --stack-len "$SIM_STACK_LEN" \
        --uart-base "$SIM_UART_BASE" \
        --max-cycles "$SIM_MAX_CYCLES" \
        "$@"
}

SSEOCB_G_VPBASE=0xfff00000
SSEOCB_G_VPSIZE=0x00100000

# Cleanup
rm -rf artifacts main.o pre.o pre.map bridge.o int.o final.o final.bin
mkdir -p artifacts

# Export memory.ld
sim export-script > memory.ld

# Compile main.c as rv32e
clang --target=riscv32 \
      -march=rv32e \
      -mabi=ilp32e \
      -mno-relax \
      -ffreestanding \
      -nostdlib \
      -O0 \
      -c main.c -o artifacts/main.o

# Generate pre artifacts
./tool pre artifacts/main.o artifacts --vpbase $SSEOCB_G_VPBASE --vpsize $SSEOCB_G_VPSIZE --entry _start

# Promote to rv32i
./tool promote artifacts/main.o artifacts/main.o

# Link to get pre.o
clang --target=riscv32 \
      -march=rv32i \
      -mabi=ilp32 \
      -mno-relax \
      -nostdlib \
      -Wl,-T,memory.ld \
      -Wl,-T,board.ld \
      -Wl,-T,artifacts/virtual.ld \
      -Wl,-T,artifacts/plt.ld \
      -Wl,@artifacts/plt.wrap \
      -Wl,--unresolved-symbols=ignore-all \
      -Wl,-e,0 \
      -Wl,-Map=artifacts/pre.map \
      artifacts/main.o \
      artifacts/trampolines.s \
      -o artifacts/pre.o

# Generate final artifacts and vP_raw
./tool post artifacts/pre.o artifacts

cat artifacts/pre.map
llvm-nm artifacts/pre.o

# Build int
cargo +nightly build -p int --release --target riscv32e-unknown-none-elf -Z build-std=core,compiler_builtins

llvm-nm --print-size --size-sort --radix=d --demangle ../target/riscv32e-unknown-none-elf/release/libint.rlib

llvm-objdump -d --demangle ../target/riscv32e-unknown-none-elf/release/libint.rlib

# Generate int.o
ld.lld -r --gc-sections -u __sseocb_i_main ../target/riscv32e-unknown-none-elf/release/libint.a -o artifacts/int.o

# Remap registers and promote to rv32i
./tool remap artifacts/int.o artifacts/int.o
./tool promote artifacts/int.o artifacts/int.o rv32i

# Assemble bridge.s as rv32i
clang --target=riscv32 \
      -march=rv32i \
      -mabi=ilp32 \
      -mno-relax \
      -ffreestanding \
      -nostdlib \
      -c ../crates/int/src/bridge.s -o artifacts/bridge.o

# Link final binary
clang --target=riscv32 \
      -march=rv32i \
      -mabi=ilp32 \
      -mno-relax \
      -nostdlib \
      -Wl,-T,memory.ld \
      -Wl,-T,board.ld \
      -Wl,-T,artifacts/virtual.ld \
      -Wl,-T,artifacts/plt.ld \
      -Wl,@artifacts/plt.wrap \
      -Wl,-e,__sseocb_i_entry \
      -Wl,--gc-sections \
      -Wl,-Map=artifacts/final.map \
      artifacts/main.o \
      artifacts/trampolines.s \
      artifacts/plt_vP.s \
      artifacts/vP.s \
      artifacts/bridge.o \
      artifacts/int.o \
      -o artifacts/final.o

echo "=== artifacts/final.o created successfully ==="
llvm-readelf -a artifacts/final.o
llvm-size -A -d artifacts/final.o

# Extract flat binary and run in simulator
llvm-objcopy -O binary -R .__sseocb_g.vP artifacts/final.o artifacts/final.bin
ENTRY_ADDR=$(llvm-readelf -h artifacts/final.o | grep "Entry point address:" | awk '{print $4}')
echo "=== Running in sim (entry: $ENTRY_ADDR) ==="
sim debug --elf artifacts/final.o --entry "$ENTRY_ADDR" artifacts/final.bin
echo "=== Simulation passed successfully! ==="
