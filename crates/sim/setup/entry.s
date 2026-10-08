.section .text._start
.global _start

_start:
    la sp, _stack_top
    andi sp, sp, -16

    # Copy .data section from FLASH (LMA) to RAM (VMA)
    la a0, _sidata
    la a1, _sdata
    la a2, _edata
1:
    bgeu a1, a2, 2f
    lw t0, 0(a0)
    sw t0, 0(a1)
    addi a0, a0, 4
    addi a1, a1, 4
    j 1b
2:

    # Zero .bss section in RAM
    la a0, _sbss
    la a1, _ebss
3:
    bgeu a0, a1, 4f
    sw zero, 0(a0)
    addi a0, a0, 4
    j 3b
4:

    call main
    ebreak
1:  j 1b
