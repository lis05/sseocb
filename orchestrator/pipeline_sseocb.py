import os
from pathlib import Path
import re
import shutil
from typing import Callable, Dict, Tuple
from orchestrator.config import Config
from orchestrator.pipeline_native import compile_rv32e_if_source, get_region_aliases
from orchestrator.toolchain import Toolchain, ToolchainError

CompressorFn = Callable[[Path, Path, Toolchain, Config], Path]


def compress_noop(raw_bin: Path, out_dir: Path, toolchain: Toolchain, config: Config) -> Path:
    """No-op compression hook: leaves payload uncompressed."""
    compressed_bin = out_dir / "vP_compressed.bin"
    shutil.copyfile(raw_bin, compressed_bin)
    return compressed_bin


def compress_blaris(raw_bin: Path, out_dir: Path, toolchain: Toolchain, config: Config) -> Path:
    """Blaris compression hook using tool compress-blaris."""
    compressed_bin = out_dir / "vP_compressed.bin"
    toolchain.run([
        toolchain.tool,
        "compress-blaris",
        raw_bin.resolve(),
        compressed_bin.resolve(),
    ])
    return compressed_bin


def compress_graemi(raw_bin: Path, out_dir: Path, toolchain: Toolchain, config: Config) -> Path:
    """Graemi compression hook using tool compress-graemi."""
    compressed_bin = out_dir / "vP_compressed.bin"
    toolchain.run([
        toolchain.tool,
        "compress-graemi",
        raw_bin.resolve(),
        compressed_bin.resolve(),
    ])
    return compressed_bin


COMPRESSORS: Dict[str, CompressorFn] = {
    "noop": compress_noop,
    "blaris": compress_blaris,
    "graemi": compress_graemi,
}


def register_compressor(name: str, compressor_fn: CompressorFn) -> None:
    COMPRESSORS[name.lower()] = compressor_fn


def compress_payload(
    config: Config,
    toolchain: Toolchain,
    raw_payload: Path,
    out_dir: Path,
) -> Path:
    """Runs the compression hook matching config.decompressor and returns the compressed payload path."""
    dec_norm = config.decompressor.lower()
    compressor = COMPRESSORS.get(dec_norm)
    if not compressor:
        known = ", ".join(sorted(COMPRESSORS.keys()))
        raise ToolchainError(
            f"Unsupported decompressor/compressor '{config.decompressor}'. Supported: {known}"
        )
    return compressor(raw_payload, out_dir, toolchain, config)


def generate_vp_assembly(payload_bin: Path, vp_s: Path) -> None:
    """Generates the assembly wrapper embedding the payload into section .__sseocb_i.vP."""
    payload_abs = payload_bin.resolve()
    vp_s.write_text(f"""\
.section .__sseocb_i.vP, "a", @progbits
.globl __sseocb_i_vpbase
.globl __sseocb_i_vpend
.p2align 2
__sseocb_i_vpbase:
    .incbin "{payload_abs}"
__sseocb_i_vpend:
""")


def build_sseocb(config: Config, toolchain: Toolchain) -> Tuple[Path, str, Path]:
    out = config.output_dir
    out.mkdir(parents=True, exist_ok=True)

    guest_obj = compile_rv32e_if_source(
        config.guest_obj,
        out / "guest-compiled.o",
        config,
        toolchain,
        is_guest=True,
    )


    # 1. Pre-pass
    toolchain.run([
        toolchain.tool,
        "pre",
        guest_obj,
        out,
        "--vpbase",
        f"0x{config.vpbase:08x}",
        "--vpsize",
        f"0x{config.vpsize:x}",
        "--entry",
        config.entry,
    ])

    # 2. Promote guest object to rv32i
    guest_promoted = out / "guest-promoted.o"
    toolchain.run([
        toolchain.tool,
        "promote",
        guest_obj,
        guest_promoted,
        "rv32i",
    ])

    # 3. Promote guest native object to rv32i if present
    native_promoted = None
    if config.guest_native_obj:
        native_obj = compile_rv32e_if_source(
            config.guest_native_obj,
            out / "guest-native-compiled.o",
            config,
            toolchain,
        )
        native_promoted = out / "guest-native-promoted.o"
        toolchain.run([
            toolchain.tool,
            "promote",
            native_obj,
            native_promoted,
            "rv32i",
        ])

    # 4. Generate board linker script
    board_ld = out / "board.ld"
    aliases = get_region_aliases(config.memory_ld)
    board_ld.write_text(f"""\
{aliases}
SECTIONS
{{
    .rodata : {{
        . = ALIGN(4);
        KEEP(*(.rodata .rodata.* .srodata .srodata.*))
        . = ALIGN(4);
    }} > FLASH

    .__sseocb_g.text.plt : {{
        . = ALIGN(4);
        KEEP(*(.__sseocb_g.text.plt))
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

    .__sseocb_g.text.native : {{
        . = ALIGN(4);
        KEEP(*guest-native-promoted.o(.text .text.*))
        . = ALIGN(4);
    }} > FLASH

    .__sseocb_i.decompressor : {{
        . = ALIGN(4);
        KEEP(*(.__sseocb_i.decompressor .__sseocb_i.decompressor.*))
        . = ALIGN(4);
    }} > FLASH

    .__sseocb_i.text : {{
        . = ALIGN(4);
        KEEP(*(.__sseocb_i.text .__sseocb_i.text.*))
        . = ALIGN(4);
    }} > FLASH

    .__sseocb_i.bridge : {{
        . = ALIGN(4);
        KEEP(*(__sseocb_i.bridge))
        . = ALIGN(4);
    }} > FLASH

    .__sseocb_i.rodata : {{
        . = ALIGN(4);
        KEEP(*(.__sseocb_i.rodata .__sseocb_i.rodata.*))
        . = ALIGN(4);
    }} > FLASH

    .__sseocb_i.data : {{
        . = ALIGN(4);
        KEEP(*(.__sseocb_i.data .__sseocb_i.data.*))
        . = ALIGN(4);
    }} > RAM AT > FLASH
    _i_sdata = ADDR(.__sseocb_i.data);
    _i_edata = ADDR(.__sseocb_i.data) + SIZEOF(.__sseocb_i.data);
    _i_sidata = LOADADDR(.__sseocb_i.data);

    .__sseocb_i.bss : {{
        . = ALIGN(4);
        KEEP(*(.__sseocb_i.bss .__sseocb_i.bss.*))
        . = ALIGN(4);
    }} > RAM
    _i_sbss = ADDR(.__sseocb_i.bss);
    _i_ebss = ADDR(.__sseocb_i.bss) + SIZEOF(.__sseocb_i.bss);

    .__sseocb_i.vP : {{
        . = ALIGN(4);
        KEEP(*(.__sseocb_i.vP))
        . = ALIGN(4);
    }} > FLASH

    _i_stack_top = ORIGIN(STACK) + LENGTH(STACK);
    _g_stack_top = _i_stack_top - 1024;
}}
""")

    trampolines_s = out / "trampolines.s"
    virtual_ld = out / "virtual.ld"
    plt_ld = out / "plt.ld"
    plt_wrap = out / "plt.wrap"

    # 5. Pre-link to generate pre.o
    pre_o = out / "pre.o"
    pre_map = out / "pre.map"

    pre_link_cmd = [
        toolchain.clang,
        "--target=riscv32",
        "-march=rv32i",
        "-mabi=ilp32",
        "-mno-relax",
        "-nostdlib",
        f"-Wl,-T,{config.memory_ld.resolve()}",
        f"-Wl,-T,{board_ld.resolve()}",
    ]

    if config.guest_ld:
        pre_link_cmd.append(f"-Wl,-T,{config.guest_ld.resolve()}")
    if config.guest_native_ld:
        pre_link_cmd.append(f"-Wl,-T,{config.guest_native_ld.resolve()}")

    pre_link_cmd.extend([
        f"-Wl,-T,{virtual_ld.resolve()}",
        f"-Wl,-T,{plt_ld.resolve()}",
        f"-Wl,@{plt_wrap.resolve()}",
    ])

    pre_link_cmd.extend([
        "-Wl,--unresolved-symbols=ignore-all",
        "-Wl,-e,0",
        f"-Wl,-Map={pre_map.resolve()}",
        guest_promoted,
        trampolines_s,
    ])

    if native_promoted:
        pre_link_cmd.append(native_promoted)

    pre_link_cmd.extend(config.guest_linker_flags)
    pre_link_cmd.extend(config.guest_native_linker_flags)
    pre_link_cmd.extend(["-o", pre_o])

    toolchain.run(pre_link_cmd)

    # 6. Post-pass: extract payload & generate plt_vP.s and vP.s
    toolchain.run([
        toolchain.tool,
        "post",
        pre_o,
        out,
    ])

    # 7. Compression hook: compress payload and update vP.s
    raw_payload = out / "vP_raw.bin"
    compressed_payload = compress_payload(config, toolchain, raw_payload, out)
    vp_s = out / "vP.s"
    generate_vp_assembly(compressed_payload, vp_s)

    # 8. Build interpreter
    int_a = toolchain.ensure_int_lib(decompressor=config.decompressor)
    int_ld = toolchain.repo_root / "crates" / "int" / "int.ld"
    int_o = out / "int.o"

    toolchain.run([
        toolchain.ld_lld,
        "-r",
        "-T",
        int_ld,
        "--gc-sections",
        "-u",
        "__sseocb_i_main",
        int_a,
        "-o",
        int_o,
    ])

    # Localize internal interpreter symbols (e.g. compiler_builtins like __mulsi3)
    # so they do not collide with or get overridden by guest/native symbols.
    toolchain.run([
        toolchain.llvm_objcopy,
        "--wildcard",
        "--keep-global-symbol",
        "__sseocb_*",
        int_o,
    ])

    # 9. Remap interpreter registers
    int_remapped = out / "int-remapped.o"
    toolchain.run([
        toolchain.tool,
        "remap",
        int_o,
        int_remapped,
    ])

    # 10. Promote interpreter to rv32i
    int_promoted = out / "int-promoted.o"
    toolchain.run([
        toolchain.tool,
        "promote",
        int_remapped,
        int_promoted,
        "rv32i",
    ])

    # 11. Assemble bridge
    bridge_s = toolchain.repo_root / "crates" / "int" / "src" / "bridge.s"
    bridge_o = out / "bridge.o"
    toolchain.run([
        toolchain.clang,
        "--target=riscv32",
        "-march=rv32i",
        "-mabi=ilp32",
        "-mno-relax",
        "-ffreestanding",
        "-nostdlib",
        "-c",
        bridge_s,
        "-o",
        bridge_o,
    ])

    # 12. Final link
    final_o = out / "final.o"
    final_map = out / "final.map"
    plt_vp_s = out / "plt_vP.s"
    vp_s = out / "vP.s"

    final_link_cmd = [
        toolchain.clang,
        "--target=riscv32",
        "-march=rv32i",
        "-mabi=ilp32",
        "-mno-relax",
        "-nostdlib",
        f"-Wl,-T,{config.memory_ld.resolve()}",
        f"-Wl,-T,{board_ld.resolve()}",
    ]

    if config.guest_ld:
        final_link_cmd.append(f"-Wl,-T,{config.guest_ld.resolve()}")
    if config.guest_native_ld:
        final_link_cmd.append(f"-Wl,-T,{config.guest_native_ld.resolve()}")

    final_link_cmd.extend([
        f"-Wl,-T,{virtual_ld.resolve()}",
        f"-Wl,-T,{plt_ld.resolve()}",
        f"-Wl,@{plt_wrap.resolve()}",
    ])

    final_link_cmd.extend([
        "-Wl,-e,__sseocb_i_entry",
        f"-Wl,-Map={final_map.resolve()}",
        guest_promoted,
        trampolines_s,
        plt_vp_s,
        vp_s,
        bridge_o,
        int_promoted,
    ])

    if native_promoted:
        final_link_cmd.append(native_promoted)

    final_link_cmd.extend(config.guest_linker_flags)
    final_link_cmd.extend(config.guest_native_linker_flags)
    final_link_cmd.extend(["-o", final_o])

    toolchain.run(final_link_cmd)

    # 13. Extract flat binary
    final_bin = out / "final.bin"
    toolchain.run([
        toolchain.llvm_objcopy,
        "-O",
        "binary",
        "-R",
        ".__sseocb_g.vP",
        final_o,
        final_bin,
    ])

    # 14. Extract entry address
    readelf_res = toolchain.run([
        toolchain.llvm_readelf,
        "-h",
        final_o,
    ])
    match = re.search(r"Entry point address:\s+(0x[0-9a-fA-F]+|\d+)", readelf_res.stdout)
    if not match:
        raise ToolchainError(f"Could not parse entry point address from readelf output:\n{readelf_res.stdout}")
    entry_addr = match.group(1)

    return final_bin, entry_addr, final_o
