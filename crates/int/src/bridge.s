.section __sseocb_i.bridge, "ax", @progbits

# int operates in the top half of rv32i registers (x0, x16..x31) where it sees
# x16..x30 as its x1..x15. x31 is scratch. Offset is 15.

.globl __sseocb_i_entry
__sseocb_i_entry:
    # Setup guest stack
    la x2, _g_stack_top
    andi x2, x2, -16

    # Setup int stack
    la x17, _i_stack_top
    andi x17, x17, -16

    # Setup .data
    la x28, _sidata
    la x29, _sdata
    la x30, _edata
    1:
        bgeu x29, x30, 2f
        lw x31, 0(x28)
        sw x31, 0(x29)
        addi x28, x28, 4
        addi x29, x29, 4
        j 1b

    2:

    # Clear .bss
    la x28, _sbss
    la x29, _ebss
    3:
        bgeu x28, x29, 4f
        sw x0, 0(x28)
        addi x28, x28, 4
        j 3b
    4:

    # Setup interpreter .data
    la x28, _i_sidata
    la x29, _i_sdata
    la x30, _i_edata
    5:
        bgeu x29, x30, 6f
        lw x31, 0(x28)
        sw x31, 0(x29)
        addi x28, x28, 4
        addi x29, x29, 4
        j 5b
    6:

    # Clear interpreter .bss
    la x28, _i_sbss
    la x29, _i_ebss
    7:
        bgeu x28, x29, 8f
        sw x0, 0(x28)
        addi x28, x28, 4
        j 7b
    8:

    # Initialize vPC
    la x25, __sseocb_g_entry

    # Init x1 for safe return
    la x1, 9f

    # Jump to main
    j __sseocb_i_main

    9:
    ebreak
    10:
    j 10b

.globl __sseocb_i_read_guest_reg
__sseocb_i_read_guest_reg:
    # __sseocb_i_read_guest_reg(reg_index) reads a guest register and returns it.
    #
    # reg_index is in x10+15=x25
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
    # reg_index is in x10+15=x25
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

.globl __sseocb_i_native_call
__sseocb_i_native_call:
    # __sseocb_i_native_call(addr) executes a native call into guest code.
    #
    # addr is in x10+15=x25
    # Return address is in x1+15=x16
    la x1, 1f
    jr x25
    1:
    jr x16

.globl __sseocb_i_plt_call
__sseocb_i_plt_call:
    # __sseocb_i_plt_call transfers control from native code to the intepreter.
    #
    # Incoming addr is in x31, plt_slot is x31 - __sseocb_i_trampolines - 4
    # Return address is in x1 (native)
    #
    # Guest stack: x2
    # Int stack: x17

    # Before transfering control to the interpreter, save:
    # x16..x30 without x17 (14x4=56 bytes)
    # x1
    addi x17, x17, -64
    sw x16, 0(x17)
    sw x18, 4(x17)
    sw x19, 8(x17)
    sw x20, 12(x17)
    sw x21, 16(x17)
    sw x22, 20(x17)
    sw x23, 24(x17)
    sw x24, 28(x17)
    sw x25, 32(x17)
    sw x26, 36(x17)
    sw x27, 40(x17)
    sw x28, 44(x17)
    sw x29, 48(x17)
    sw x30, 52(x17)
    sw x1, 56(x17)

    # Return to the epilogue
    la x1, __sseocb_i_plt_return

    # Calculate plt slot
    la x30, __sseocb_i_trampolines + 4 #  4 because jal stores PC+4
    sub x31, x31, x30

    # Load vPC into x10+15=x25
    la x30, __sseocb_i_plt_offsets
    add x25, x31, x30 # slot + base
    lw x25, 0(x25)

    # Interpreter will detect plt_return and jump back here
    jal x16, __sseocb_i_main
    # Fallthrough intended

.globl __sseocb_i_plt_return
__sseocb_i_plt_return:
    # __sseocb_i_plt_return transfers control back to native code from interpreter.

    # Restore int state and x1
    lw x16, 0(x17)
    lw x18, 4(x17)
    lw x19, 8(x17)
    lw x20, 12(x17)
    lw x21, 16(x17)
    lw x22, 20(x17)
    lw x23, 24(x17)
    lw x24, 28(x17)
    lw x25, 32(x17)
    lw x26, 36(x17)
    lw x27, 40(x17)
    lw x28, 44(x17)
    lw x29, 48(x17)
    lw x30, 52(x17)
    lw x1, 56(x17)
    addi x17, x17, 64

    # Return to native
    jr x1
