clang --target=riscv32 \
      -march=rv32e \
      -mabi=ilp32e \
      -mno-relax \
      -ffreestanding \
      -nostdlib \
      -O0 \
      -c main.c -o main.o

