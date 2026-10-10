.section .text.__sseocb_i_bridge, "ax", @progbits

# int operates in the top half of rv32i registers (x0, x16..x31) where it sees
# x16..x30 as its x1..x15. x31 is scratch. Offset is 15.

.globl __sseocb_i_read_guest_reg
__sseocb_i_read_guest_reg:
    # __sseocb_i_read_guest_reg(reg_index) reads a guest register and returns it.
    #
    # read_index is in x10+15=x25
    # Return register is x10+15=x25
    # Return address is in x1+15=x16

    la x31, 1f
    slli x25, x25, 3 # each table entry is 8 bytes (2^3 = 8)
    add x31, x31, x25
    jr x31

    .p2align 3
    1:
        mv x25, x0
        jr x16
        mv x25, x1
        jr x16
        mv x25, x2
        jr x16
        mv x25, x3
        jr x16
        mv x25, x4
        jr x16
        mv x25, x5
        jr x16
        mv x25, x6
        jr x16
        mv x25, x7
        jr x16
        mv x25, x8
        jr x16
        mv x25, x9
        jr x16
        mv x25, x10
        jr x16
        mv x25, x11
        jr x16
        mv x25, x12
        jr x16
        mv x25, x13
        jr x16
        mv x25, x14
        jr x16
        mv x25, x15
        jr x16

.globl __sseocb_i_write_guest_reg
__sseocb_i_write_guest_reg:
    # __sseocb_i_write_guest_reg(reg_index, value) writes into a guest register.
    #
    # read_index is in x10+15=x25
    # value is in x11+15=x26
    # Return address is in x1+15=x16

    la x31, 1f
    slli x25, x25, 3 # each table entry is 8 bytes (2^3 = 8)
    add x31, x31, x25
    jr x31

    .p2align 3
    1:
        mv x0, x26
        jr x16
        mv x1, x26
        jr x16
        mv x2, x26
        jr x16
        mv x3, x26
        jr x16
        mv x4, x26
        jr x16
        mv x5, x26
        jr x16
        mv x6, x26
        jr x16
        mv x7, x26
        jr x16
        mv x8, x26
        jr x16
        mv x9, x26
        jr x16
        mv x10, x26
        jr x16
        mv x11, x26
        jr x16
        mv x12, x26
        jr x16
        mv x13, x26
        jr x16
        mv x14, x26
        jr x16
        mv x15, x26
        jr x16

