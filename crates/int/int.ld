SECTIONS {
    .text.__sseocb_i.rust : { *(.text .text.*) }
    .__sseocb_i.rodata    : { *(.rodata .rodata.*) }
}
