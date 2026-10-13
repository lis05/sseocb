SECTIONS {
    .text.__sseocb_i.rust : { *(.text .text.*) }
    .__sseocb_i.rodata    : { *(.rodata .rodata.*) }
    .__sseocb_i.data      : { *(.data .data.*) }
    .__sseocb_i.bss       : { *(.bss .bss.*) *(COMMON) }
}
