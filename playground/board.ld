SECTIONS
{
    .rodata : {
        . = ALIGN(4);
        *(.rodata .rodata.*)
        . = ALIGN(4);
    } > FLASH

    .text.__sseocb_g.plt : {
        . = ALIGN(4);
        *(.text.__sseocb_g.plt)
        . = ALIGN(4);
    } > FLASH

    .text.__sseocb_i.rust : {
        . = ALIGN(4);
        *(.text.__sseocb_i.rust)
        . = ALIGN(4);
    } > FLASH

    .text.__sseocb_i.bridge : {
        . = ALIGN(4);
        *(.text.__sseocb_i.bridge)
        . = ALIGN(4);
    } > FLASH

    .__sseocb_i.rodata : {
        . = ALIGN(4);
        *(.__sseocb_i.rodata .__sseocb_i.rodata.*)
        . = ALIGN(4);
    } > FLASH

    .__sseocb_i.vP : {
        . = ALIGN(4);
        *(.__sseocb_i.vP)
        . = ALIGN(4);
    } > FLASH

    .data : {
        . = ALIGN(4);
        _sdata = .;
        *(.data .data.*)
        . = ALIGN(4);
        _edata = .;
    } > RAM AT > FLASH
    _sidata = LOADADDR(.data);

    .bss : {
        . = ALIGN(4);
        _sbss = .;
        *(.bss .bss.*)
        *(COMMON)
        . = ALIGN(4);
        _ebss = .;
    } > RAM

    .__sseocb_i.data : {
        . = ALIGN(4);
        _i_sdata = .;
        *(.__sseocb_i.data .__sseocb_i.data.*)
        . = ALIGN(4);
        _i_edata = .;
    } > RAM AT > FLASH
    _i_sidata = LOADADDR(.__sseocb_i.data);

    .__sseocb_i.bss : {
        . = ALIGN(4);
        _i_sbss = .;
        *(.__sseocb_i.bss .__sseocb_i.bss.*)
        . = ALIGN(4);
        _i_ebss = .;
    } > RAM

    _i_stack_top = ORIGIN(STACK) + LENGTH(STACK);
    _g_stack_top = _i_stack_top - 1024;
}
