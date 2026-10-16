SECTIONS {
    .__sseocb_i.decompressor        : {
        *blaris*(.text .text.*)
        *graemi*(.text .text.*)
        *(.__sseocb_i.decompressor)
    }
    .__sseocb_i.decompressor.rodata : {
        *blaris*(.rodata .rodata.*)
        *graemi*(.rodata .rodata.*)
        *(.__sseocb_i.decompressor.rodata)
    }
    .__sseocb_i.text          : { *(.text .text.*) }
    .__sseocb_i.rodata        : { *(.rodata .rodata.* .srodata .srodata.*) }
    .__sseocb_i.data          : { *(.data .data.* .sdata .sdata.*) }
    .__sseocb_i.bss           : { *(.bss .bss.* .sbss .sbss.*) *(COMMON) }
}
