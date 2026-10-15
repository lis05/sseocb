import argparse
import difflib
from pathlib import Path
import re
import sys
from typing import Dict, List, Optional

from orchestrator.config import Config
from orchestrator.pipeline_native import build_native
from orchestrator.pipeline_sseocb import build_sseocb
from orchestrator.runner import run_sim
from orchestrator.toolchain import Toolchain, ToolchainError


def parse_addr(val: str) -> int:
    val = val.strip()
    if val.startswith("0x") or val.startswith("0X"):
        return int(val, 16)
    return int(val)


def add_common_build_args(parser: argparse.ArgumentParser) -> None:
    parser.add_argument(
        "--guest-obj",
        required=True,
        type=Path,
        help="Path to guest object file (guest.o)",
    )
    parser.add_argument(
        "--memory-ld",
        required=True,
        type=Path,
        help="Path to memory regions linker script (memory.ld)",
    )
    parser.add_argument(
        "--output",
        required=True,
        type=Path,
        help="Path to output directory for all artifacts",
    )
    parser.add_argument(
        "--guest-ld",
        type=Path,
        default=None,
        help="Linker script for guest code sections",
    )
    parser.add_argument(
        "--guest-linker-flags",
        nargs="*",
        default=[],
        help="Extra flags passed when linking guest code",
    )
    parser.add_argument(
        "--guest-native-obj",
        type=Path,
        default=None,
        help="Path to uncompressed native guest object file (guest-native.o)",
    )
    parser.add_argument(
        "--guest-native-ld",
        type=Path,
        default=None,
        help="Linker script for native guest code sections",
    )
    parser.add_argument(
        "--guest-native-linker-flags",
        nargs="*",
        default=[],
        help="Extra flags passed when linking native guest code",
    )
    parser.add_argument(
        "--include-dirs",
        "-I",
        nargs="*",
        dest="include_dirs",
        default=[],
        type=Path,
        help="Include directories for compiling C sources",
    )
    parser.add_argument(
        "--c-flags",
        nargs="*",
        default=[],
        help="Extra compiler flags passed to clang when compiling C sources",
    )
    parser.add_argument(
        "--entry",
        default="_start",
        help="Entry point symbol (default: _start)",
    )
    parser.add_argument(
        "--vpbase",
        default="0xfff00000",
        help="Base virtual address of vP (default: 0xfff00000)",
    )
    parser.add_argument(
        "--vpsize",
        default="0x00100000",
        help="Virtual address size of vP (default: 0x00100000)",
    )
    parser.add_argument(
        "--decompressor",
        default="noop",
        help="Decompressor implementation (default: noop)",
    )
    parser.add_argument(
        "--blaris",
        dest="decompressor",
        action="store_const",
        const="blaris",
        help="Use blaris decompressor (shorthand for --decompressor blaris)",
    )
    parser.add_argument(
        "--noop",
        dest="decompressor",
        action="store_const",
        const="noop",
        help="Use noop decompressor (shorthand for --decompressor noop)",
    )
    parser.add_argument(
        "--log",
        action="store_true",
        help="Print every command executed by orchestrator",
    )
    parser.add_argument(
        "--size",
        action="store_true",
        help="Print detailed memory size breakdown",
    )


def add_common_run_args(parser: argparse.ArgumentParser) -> None:
    add_common_build_args(parser)
    parser.add_argument(
        "--print-output",
        "--show-output",
        dest="print_output",
        action="store_true",
        help="Print UART output to stdout",
    )
    parser.add_argument(
        "--debug",
        action="store_true",
        help="Launch interactive simulator debugger (sim debug)",
    )
    parser.add_argument(
        "--expected-output",
        type=Path,
        default=None,
        help="Path to expected UART output file to verify against",
    )
    parser.add_argument(
        "--max-cycles",
        type=int,
        default=100_000_000,
        help="Max simulation cycle limit (default: 100M)",
    )


def verify_output(actual: str, expected_path: Optional[Path]) -> None:
    if expected_path is None:
        return
    if not expected_path.exists():
        raise ToolchainError(f"Expected output file not found: {expected_path}")
    expected = expected_path.read_text()
    if actual != expected:
        diff = "\n".join(
            difflib.unified_diff(
                expected.splitlines(),
                actual.splitlines(),
                fromfile="expected",
                tofile="actual",
                lineterm="",
            )
        )
        raise ToolchainError(f"Output verification failed!\nDiff:\n{diff}")


def build_config(args: argparse.Namespace, subcommand: str, target: str) -> Config:
    return Config(
        subcommand=subcommand,
        target=target,
        guest_obj=args.guest_obj,
        memory_ld=args.memory_ld,
        output_dir=args.output,
        guest_ld=args.guest_ld,
        guest_linker_flags=args.guest_linker_flags,
        guest_native_obj=args.guest_native_obj,
        guest_native_ld=args.guest_native_ld,
        guest_native_linker_flags=args.guest_native_linker_flags,
        include_dirs=getattr(args, "include_dirs", []),
        c_flags=getattr(args, "c_flags", []),
        entry=args.entry,
        vpbase=parse_addr(args.vpbase),
        vpsize=parse_addr(args.vpsize),
        decompressor=args.decompressor,
        print_output=getattr(args, "print_output", False),
        debug=getattr(args, "debug", False),
        expected_output=getattr(args, "expected_output", None),
        max_cycles=getattr(args, "max_cycles", 100_000_000),
        log=getattr(args, "log", False),
        size=getattr(args, "size", False),
    )


def parse_elf_sections(toolchain: Toolchain, elf_path: Path) -> Dict[str, int]:
    res = toolchain.run([toolchain.llvm_readelf, "-S", elf_path])
    pattern = re.compile(
        r"^\s*\[\s*(\d+)\]\s+(\S+)\s+([A-Z_]+)\s+[0-9a-fA-F]+\s+[0-9a-fA-F]+\s+([0-9a-fA-F]+)",
        re.M,
    )
    sections = {}
    for m in pattern.finditer(res.stdout):
        _idx, sec_name, _sec_type, sec_size = m.groups()
        sections[sec_name] = int(sec_size, 16)
    return sections


def print_size_report(toolchain: Toolchain, elf_path: Path) -> None:
    try:
        display_path = elf_path.relative_to(Path.cwd())
    except ValueError:
        display_path = elf_path

    res = toolchain.run([toolchain.llvm_size, "-A", str(display_path)])
    lines = res.stdout.splitlines()
    if len(lines) < 2:
        return

    ignored = {".comment", ".riscv.attributes"}
    new_lines = []
    total_size = 0
    orig_vp: Optional[int] = None
    comp_vp: Optional[int] = None
    header_len = 0

    for line in lines:
        parts = line.split()
        if not parts:
            continue
        if parts[0] in ignored:
            continue
        if parts[0] == "section":
            header_len = len(line)
            new_lines.append(line)
            continue
        if parts[0] == "Total" and len(parts) >= 2:
            old_tot = parts[1]
            prefix = line[:line.rfind(old_tot)]
            width = len(parts[1]) + len(prefix) - len("Total")
            new_lines.append(f"Total{total_size:>{width}}")
            continue

        if len(parts) >= 3 and parts[0] != "section":
            name = parts[0]
            try:
                sz = int(parts[1])
                total_size += sz
                if name == ".__sseocb_g.vP":
                    orig_vp = sz
                elif name == ".__sseocb_i.vP":
                    comp_vp = sz

                idx_sz = line.find(parts[1], len(parts[0]))
                end_sz = idx_sz + len(parts[1])
                prefix = line[:end_sz]
                addr_val = int(parts[2])
                addr_hex = f"0x{addr_val:x}" if addr_val != 0 else "0"
                addr_field_width = max(len(addr_hex) + 1, header_len - end_sz) if header_len > end_sz else len(addr_hex) + 2
                new_lines.append(f"{prefix}{addr_hex:>{addr_field_width}}")
                continue
            except ValueError:
                pass
        new_lines.append(line)

    print("\n".join(new_lines))

def print_sseocb_size_report(
    toolchain: Toolchain,
    sseocb_elf: Path,
    native_elf_path: Optional[Path] = None,
) -> None:
    print_size_report(toolchain, sseocb_elf)


def print_native_size_report(toolchain: Toolchain, native_elf: Path) -> None:
    print_size_report(toolchain, native_elf)


def main(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(
        prog="o",
        description="SSEOCB Orchestrator Tool",
    )
    subparsers = parser.add_subparsers(dest="subcommand", required=True)

    # o build ...
    build_parser = subparsers.add_parser("build", help="Build binaries")
    build_sub = build_parser.add_subparsers(dest="target", required=True)

    build_native_parser = build_sub.add_parser("native", help="Build native binary")
    add_common_build_args(build_native_parser)

    build_sseocb_parser = build_sub.add_parser("sseocb", help="Build SSEOCB binary")
    add_common_build_args(build_sseocb_parser)

    # o run ...
    run_parser = subparsers.add_parser("run", help="Run binaries in simulator")
    run_sub = run_parser.add_subparsers(dest="target", required=True)

    run_native_parser = run_sub.add_parser("native", help="Run native binary")
    add_common_run_args(run_native_parser)

    run_sseocb_parser = run_sub.add_parser("sseocb", help="Run SSEOCB binary")
    add_common_run_args(run_sseocb_parser)

    run_bench_parser = run_sub.add_parser(
        "bench",
        help="Run both native and SSEOCB, verify outputs, and report slowdown",
    )
    add_common_run_args(run_bench_parser)

    args = parser.parse_args(argv)
    config = build_config(args, args.subcommand, args.target)
    toolchain = Toolchain(log=config.log)

    try:
        if config.subcommand == "build" and config.target == "native":
            bin_path, entry_addr, elf_path = build_native(config, toolchain)
            print(f"Native binary built: {bin_path} (entry: {entry_addr})")
            if config.size:
                print_native_size_report(toolchain, elf_path)
            return 0

        if config.subcommand == "build" and config.target == "sseocb":
            bin_path, entry_addr, elf_path = build_sseocb(config, toolchain)
            print(f"SSEOCB binary built: {bin_path} (entry: {entry_addr})")
            if config.size:
                print_sseocb_size_report(toolchain, elf_path)
            return 0

        if config.subcommand == "run" and config.target == "native":
            bin_path, entry_addr, elf_path = build_native(config, toolchain)
            sim_res = run_sim(
                toolchain,
                config.memory_ld,
                bin_path,
                entry=entry_addr,
                elf_path=elf_path,
                max_cycles=config.max_cycles,
                debug=config.debug,
                stream_output=config.print_output,
            )
            if config.print_output and not sim_res.streamed:
                print(sim_res.stdout, end="")
            if not config.debug:
                print(f"Native cycles: {sim_res.cycles}")
                verify_output(sim_res.stdout, config.expected_output)
            if config.size:
                print_native_size_report(toolchain, elf_path)
            return 0

        if config.subcommand == "run" and config.target == "sseocb":
            bin_path, entry_addr, elf_path = build_sseocb(config, toolchain)
            sim_res = run_sim(
                toolchain,
                config.memory_ld,
                bin_path,
                entry=entry_addr,
                elf_path=elf_path,
                max_cycles=config.max_cycles,
                debug=config.debug,
                stream_output=config.print_output,
            )
            if config.print_output and not sim_res.streamed:
                print(sim_res.stdout, end="")
            if not config.debug:
                print(f"SSEOCB cycles: {sim_res.cycles}")
                verify_output(sim_res.stdout, config.expected_output)
            if config.size:
                print_sseocb_size_report(toolchain, elf_path)
            return 0

        if config.subcommand == "run" and config.target == "bench":
            # 1. Run Native (quiet)
            native_bin, native_entry, native_elf = build_native(config, toolchain)
            native_res = run_sim(
                toolchain,
                config.memory_ld,
                native_bin,
                entry=native_entry,
                elf_path=native_elf,
                max_cycles=config.max_cycles,
                debug=False,
                stream_output=False,
            )

            # 2. Run SSEOCB (stream live if print_output is enabled)
            sseocb_bin, entry_addr, elf_path = build_sseocb(config, toolchain)
            sseocb_res = run_sim(
                toolchain,
                config.memory_ld,
                sseocb_bin,
                entry=entry_addr,
                elf_path=elf_path,
                max_cycles=config.max_cycles,
                debug=False,
                stream_output=config.print_output,
            )

            if config.print_output and not sseocb_res.streamed:
                print(sseocb_res.stdout, end="")

            # 3. Output verification
            verify_output(native_res.stdout, config.expected_output)
            verify_output(sseocb_res.stdout, config.expected_output)

            if native_res.stdout != sseocb_res.stdout:
                diff = "\n".join(
                    difflib.unified_diff(
                        native_res.stdout.splitlines(),
                        sseocb_res.stdout.splitlines(),
                        fromfile="native",
                        tofile="sseocb",
                        lineterm="",
                    )
                )
                raise ToolchainError(
                    f"Mismatch between native and SSEOCB outputs!\nDiff:\n{diff}"
                )

            # 4. Report Slowdown
            slowdown = (
                sseocb_res.cycles / native_res.cycles
                if native_res.cycles > 0
                else 0.0
            )

            print("=== Benchmark Results ===")
            print(f"Native cycles:  {native_res.cycles:,}")
            print(f"SSEOCB cycles:  {sseocb_res.cycles:,}")
            print(f"Slowdown:       {slowdown:.2f}x")
            print("Output verify:  PASSED")
            if config.size:
                print_sseocb_size_report(toolchain, elf_path, native_elf_path=native_elf)
            return 0

        raise ToolchainError(f"Unknown subcommand/target: {config.subcommand} {config.target}")

    except ToolchainError as e:
        print(f"Error: {e}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
