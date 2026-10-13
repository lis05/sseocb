import argparse
import difflib
from pathlib import Path
import sys
from typing import List, Optional

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
        "--log",
        action="store_true",
        help="Print every command executed by orchestrator",
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
    )


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
            bin_path, entry_addr, _ = build_native(config, toolchain)
            print(f"Native binary built: {bin_path} (entry: {entry_addr})")
            return 0

        if config.subcommand == "build" and config.target == "sseocb":
            bin_path, entry_addr, _ = build_sseocb(config, toolchain)
            print(f"SSEOCB binary built: {bin_path} (entry: {entry_addr})")
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
            )
            if config.print_output:
                print(sim_res.stdout, end="")
            if not config.debug:
                print(f"Native cycles: {sim_res.cycles}")
                verify_output(sim_res.stdout, config.expected_output)
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
            )
            if config.print_output:
                print(sim_res.stdout, end="")
            if not config.debug:
                print(f"SSEOCB cycles: {sim_res.cycles}")
                verify_output(sim_res.stdout, config.expected_output)
            return 0

        if config.subcommand == "run" and config.target == "bench":
            # 1. Run Native
            native_bin, native_entry, native_elf = build_native(config, toolchain)
            native_res = run_sim(
                toolchain,
                config.memory_ld,
                native_bin,
                entry=native_entry,
                elf_path=native_elf,
                max_cycles=config.max_cycles,
                debug=False,
            )

            # 2. Run SSEOCB
            sseocb_bin, entry_addr, elf_path = build_sseocb(config, toolchain)
            sseocb_res = run_sim(
                toolchain,
                config.memory_ld,
                sseocb_bin,
                entry=entry_addr,
                elf_path=elf_path,
                max_cycles=config.max_cycles,
                debug=False,
            )

            if config.print_output:
                print(native_res.stdout, end="")

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
            return 0

        raise ToolchainError(f"Unknown subcommand/target: {config.subcommand} {config.target}")

    except ToolchainError as e:
        print(f"Error: {e}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
