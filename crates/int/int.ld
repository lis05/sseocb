SECTIONS {
    .__sseocb_i.decompressor        : {
        *blaris*(.text .text.*)
        *(.__sseocb_i.decompressor)
    }
    .__sseocb_i.decompressor.rodata : {
        *blaris*(.rodata .rodata.*)
        *(.__sseocb_i.decompressor.rodata)
    }
    .__sseocb_i.text          : { *(.text .text.*) }
    .__sseocb_i.rodata        : { *(.rodata .rodata.*) }
    .__sseocb_i.data          : { *(.data .data.*) }
    .__sseocb_i.bss           : { *(.bss .bss.*) *(COMMON) }
}
