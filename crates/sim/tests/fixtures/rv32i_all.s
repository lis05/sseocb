# Fixture: exercises every RV32I base instruction once.
# Used by tests/parse.rs (`sim accept` must decode every word).
# Rebuild with: scripts/rv32i_compile_bin_asm.sh crates/sim/tests/fixtures/rv32i_all.s

    .section .text.init, "ax"
    .global _start
_start:
    # U-type
    lui     t0, 0x12345
    auipc   t1, 0x1

    # I-type ALU
    addi    a0, zero, 42
    slti    a1, a0, 100
    sltiu   a2, a0, 100
    xori    a3, a0, 0x0f
    ori     a4, a0, 0x100
    andi    a5, a0, 0xff
    slli    a6, a0, 3
    srli    a7, a0, 1
    srai    s2, a0, 2

    # R-type ALU
    add     s3, a0, a1
    sub     s4, a0, a1
    sll     s5, a0, a1
    slt     s6, a0, a1
    sltu    s7, a0, a1
    xor     s8, a0, a1
    srl     s9, a0, a1
    sra     s10, a0, a1
    or      s11, a0, a1
    and     t3, a0, a1

    # Stores and loads (relative to sp)
    addi    sp, sp, -16
    sw      a0, 0(sp)
    sh      a1, 4(sp)
    sb      a2, 6(sp)
    lw      t4, 0(sp)
    lh      t5, 4(sp)
    lhu     t6, 4(sp)
    lb      s0, 6(sp)
    lbu     s1, 6(sp)
    addi    sp, sp, 16

    # Branches (all fall through to the next instruction)
    beq     a0, a1, 1f
1:  bne     a0, a1, 2f
2:  blt     a0, a1, 3f
3:  bge     a0, a1, 4f
4:  bltu    a0, a1, 5f
5:  bgeu    a0, a1, 6f
6:

    # Jumps
    jal     ra, 7f
7:  auipc   t0, 0
    jalr    zero, 12(t0)
    nop

    # System / ordering
    fence
    ecall
    ebreak
