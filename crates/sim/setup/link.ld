/* memory.ld is exported by `sim run --export-script` and discovered via -L <artifacts_dir> */
INCLUDE memory.ld

ENTRY(_start)

SECTIONS
{
    .text : {
        . = ALIGN(4);
        *(.text._start)
        *(.text .text.*)
        . = ALIGN(4);
    } > FLASH

    .rodata : {
        . = ALIGN(4);
        *(.rodata .rodata.*)
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

    _stack_top = ORIGIN(STACK) + LENGTH(STACK);
}
