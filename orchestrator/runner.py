from dataclasses import dataclass
from pathlib import Path
import re
import subprocess
from typing import Optional
from orchestrator.toolchain import Toolchain, ToolchainError


@dataclass
class SimResult:
    stdout: str
    raw_output: str
    cycles: int
    exit_code: int


def run_sim(
    toolchain: Toolchain,
    memory_ld: Path,
    bin_path: Path,
    entry: Optional[str] = None,
    elf_path: Optional[Path] = None,
    max_cycles: int = 100_000_000,
    debug: bool = False,
) -> SimResult:
    cmd = [toolchain.sim]

    if debug:
        cmd.extend(["debug", "--memory-ld", memory_ld, "--stats"])
        if entry:
            cmd.extend(["--entry", entry])
        if elf_path:
            cmd.extend(["--elf", elf_path])
        cmd.append(bin_path)

        if toolchain.log:
            import shlex
            print(f"+ {shlex.join([str(x) for x in cmd])}", flush=True)

        res = subprocess.run([str(x) for x in cmd])
        return SimResult(stdout="", raw_output="", cycles=0, exit_code=res.returncode)

    cmd.extend([
        "run",
        "--memory-ld",
        memory_ld,
        "--stats",
        "--max-cycles",
        str(max_cycles),
    ])
    if entry:
        cmd.extend(["--entry", entry])
    if elf_path:
        cmd.extend(["--elf", elf_path])
    cmd.append(bin_path)

    res = toolchain.run(cmd, check=False)
    if res.returncode != 0:
        err = f"Simulation failed with exit code {res.returncode}.\n"
        if res.stderr:
            err += f"Stderr:\n{res.stderr}\n"
        if res.stdout:
            err += f"Stdout:\n{res.stdout}\n"
        raise ToolchainError(err)

    raw = res.stdout
    cycles = 0
    cycle_match = re.search(r"Cycles executed before ebreak:\s+(\d+)", raw)
    if cycle_match:
        cycles = int(cycle_match.group(1))

    clean_output = re.sub(r"Cycles executed before ebreak:\s+\d+\n?", "", raw)

    return SimResult(
        stdout=clean_output,
        raw_output=raw,
        cycles=cycles,
        exit_code=res.returncode,
    )
