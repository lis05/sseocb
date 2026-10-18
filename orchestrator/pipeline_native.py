from pathlib import Path
import re
from typing import Tuple
from orchestrator.config import Config
from orchestrator.toolchain import Toolchain, ToolchainError


def get_region_aliases(memory_ld: Path) -> str:
    content = memory_ld.read_text()
    words = set(re.findall(r"\b[a-zA-Z0-9_]+\b", content.lower()))
    has_ram = "ram" in words
    has_sram = "sram" in words
    aliases = []
    if has_sram and not has_ram:
        aliases.append('REGION_ALIAS("RAM", SRAM);')
    if has_ram and not has_sram:
        aliases.append('REGION_ALIAS("SRAM", RAM);')
    return "\n".join(aliases) + ("\n" if aliases else "")


def compile_rv32e_if_source(
    src_or_obj: Path,
    out_obj: Path,
    config: Config,
    toolchain: Toolchain,
    is_guest: bool = False,
) -> Path:
    if src_or_obj.suffix.lower() == ".c":
        inc_args = ["-I", str(src_or_obj.parent.resolve())]
        for inc in config.include_dirs:
            inc_args.extend(["-I", str(Path(inc).resolve())])
        if is_guest:
            cmd = [
                toolchain.clang_guest,
                *inc_args,
                *config.c_flags,
                "-c",
                src_or_obj.resolve(),
                "-o",
                out_obj.resolve(),
            ]
        else:
            cmd = [
                toolchain.clang,
                "--target=riscv32",
                "-march=rv32e",
                "-mabi=ilp32e",
                "-mno-relax",
                "-ffreestanding",
                "-nostdlib",
                "-fno-optimize-sibling-calls",
                "-Oz",
                *inc_args,
                *config.c_flags,
                "-c",
                src_or_obj.resolve(),
                "-o",
                out_obj.resolve(),
            ]
        toolchain.run(cmd)
        return out_obj
    return src_or_obj


def build_native(config: Config, toolchain: Toolchain) -> Tuple[Path, str, Path]:
    out = config.output_dir
    out.mkdir(parents=True, exist_ok=True)

    guest_obj = compile_rv32e_if_source(
        config.guest_obj,
        out / "guest-compiled.o",
        config,
        toolchain,
        is_guest=True,
    )


    # 1. Promote guest.o to rv32e
    guest_promoted = out / "guest-promoted-rv32e.o"
    toolchain.run([
        toolchain.tool,
        "promote",
        guest_obj,
        guest_promoted,
        "rv32e",
    ])

    # 2. Promote guest-native.o to rv32e if provided
    native_promoted = None
    if config.guest_native_obj:
        native_obj = compile_rv32e_if_source(
            config.guest_native_obj,
            out / "guest-native-compiled.o",
            config,
            toolchain,
        )
        native_promoted = out / "guest-native-promoted-rv32e.o"
        toolchain.run([
            toolchain.tool,
            "promote",
            native_obj,
            native_promoted,
            "rv32e",
        ])

    # 3. Create native startup entry
    entry_s = out / "native-entry.s"
    entry_s.write_text(f"""\
.section .text.__sseocb_native_entry, "ax", @progbits
.global __sseocb_native_entry
__sseocb_native_entry:
    la sp, _stack_top
    andi sp, sp, -16
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
    la a0, _sbss
    la a1, _ebss
3:
    bgeu a0, a1, 4f
    sw zero, 0(a0)
    addi a0, a0, 4
    j 3b
4:
    li a0, 0
    li a1, 0
    call {config.entry}
    ebreak
1:  j 1b
""")

    entry_o = out / "native-entry.o"
    toolchain.run([
        toolchain.clang,
        "--target=riscv32",
        "-march=rv32e",
        "-mabi=ilp32e",
        "-mno-relax",
        "-ffreestanding",
        "-nostdlib",
        "-c",
        entry_s,
        "-o",
        entry_o,
    ])

    # 4. Generate native section layout
    native_sections_ld = out / "native_sections.ld"
    aliases = get_region_aliases(config.memory_ld)
    native_sections_ld.write_text(f"""\
{aliases}
SECTIONS
{{
    .text : {{
        . = ALIGN(4);
        KEEP(*(.text.__sseocb_native_entry))
        KEEP(*(.text .text.*))
        . = ALIGN(4);
    }} > FLASH

    .rodata : {{
        . = ALIGN(4);
        KEEP(*(.rodata .rodata.* .srodata .srodata.*))
        . = ALIGN(4);
    }} > FLASH

    .data : {{
        . = ALIGN(4);
        KEEP(*(.data .data.* .sdata .sdata.*))
        . = ALIGN(4);
    }} > RAM AT > FLASH
    _sdata = ADDR(.data);
    _edata = ADDR(.data) + SIZEOF(.data);
    _sidata = LOADADDR(.data);

    .bss : {{
        . = ALIGN(4);
        KEEP(*(.bss .bss.* .sbss .sbss.*))
        KEEP(*(COMMON))
        . = ALIGN(4);
    }} > RAM
    _sbss = ADDR(.bss);
    _ebss = ADDR(.bss) + SIZEOF(.bss);

    _stack_top = ORIGIN(STACK) + LENGTH(STACK);
}}
""")

    # 5. Link native binary
    native_o = out / "native-linked.o"
    native_map = out / "native.map"

    link_cmd = [
        toolchain.clang,
        "--target=riscv32",
        "-march=rv32e",
        "-mabi=ilp32e",
        "-mno-relax",
        "-nostdlib",
        f"-Wl,-T,{config.memory_ld.resolve()}",
    ]

    if config.guest_ld:
        link_cmd.append(f"-Wl,-T,{config.guest_ld.resolve()}")
    if config.guest_native_ld:
        link_cmd.append(f"-Wl,-T,{config.guest_native_ld.resolve()}")

    link_cmd.extend([
        f"-Wl,-T,{native_sections_ld.resolve()}",
        "-Wl,-e,__sseocb_native_entry",
        f"-Wl,-Map={native_map.resolve()}",
        entry_o,
        guest_promoted,
    ])

    if native_promoted:
        link_cmd.append(native_promoted)

    link_cmd.extend(config.guest_linker_flags)
    link_cmd.extend(config.guest_native_linker_flags)
    link_cmd.extend(["-o", native_o])

    toolchain.run(link_cmd)

    # 6. Extract flat binary
    native_bin = out / "native.bin"
    toolchain.run([
        toolchain.llvm_objcopy,
        "-O",
        "binary",
        native_o,
        native_bin,
    ])

    # 7. Extract entry address
    readelf_res = toolchain.run([
        toolchain.llvm_readelf,
        "-h",
        native_o,
    ])
    match = re.search(r"Entry point address:\s+(0x[0-9a-fA-F]+|\d+)", readelf_res.stdout)
    if not match:
        raise ToolchainError(f"Could not parse entry point address from readelf output:\n{readelf_res.stdout}")
    entry_addr = match.group(1)

    return native_bin, entry_addr, native_o
