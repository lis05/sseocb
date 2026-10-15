import codecs
from dataclasses import dataclass
import os
from pathlib import Path
import re
import selectors
import shlex
import subprocess
import sys
from typing import Optional
from orchestrator.toolchain import Toolchain, ToolchainError


@dataclass
class SimResult:
    stdout: str
    raw_output: str
    cycles: int
    exit_code: int
    streamed: bool = False


class LiveFilter:
    PREFIX = "Cycles executed before ebreak:"

    def __init__(self, stream_to_stdout: bool = True):
        self.stream_to_stdout = stream_to_stdout
        self.pending = ""
        self.at_line_start = True
        self.in_stats_line = False

    def feed(self, text: str):
        if not self.stream_to_stdout:
            return
        for c in text:
            if self.in_stats_line:
                if c == "\n":
                    self.in_stats_line = False
                    self.at_line_start = True
                continue
            if self.at_line_start:
                candidate = self.pending + c
                if candidate == self.PREFIX[:len(candidate)]:
                    self.pending = candidate
                    if self.pending == self.PREFIX:
                        self.in_stats_line = True
                        self.pending = ""
                    continue
                else:
                    sys.stdout.write(candidate)
                    sys.stdout.flush()
                    self.pending = ""
                    self.at_line_start = (c == "\n")
            else:
                sys.stdout.write(c)
                sys.stdout.flush()
                self.at_line_start = (c == "\n")

    def finish(self):
        if self.stream_to_stdout and self.pending:
            sys.stdout.write(self.pending)
            sys.stdout.flush()
            self.pending = ""


def run_sim(
    toolchain: Toolchain,
    memory_ld: Path,
    bin_path: Path,
    entry: Optional[str] = None,
    elf_path: Optional[Path] = None,
    max_cycles: int = 100_000_000,
    debug: bool = False,
    stream_output: bool = False,
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
            print(f"+ {shlex.join([str(x) for x in cmd])}", flush=True)

        res = subprocess.run([str(x) for x in cmd])
        return SimResult(stdout="", raw_output="", cycles=0, exit_code=res.returncode, streamed=False)

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

    if toolchain.log:
        print(f"+ {shlex.join([str(x) for x in cmd])}", flush=True)

    proc = subprocess.Popen(
        [str(x) for x in cmd],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        bufsize=0,
    )

    stdout_decoder = codecs.getincrementaldecoder("utf-8")("replace")
    stderr_decoder = codecs.getincrementaldecoder("utf-8")("replace")
    stdout_chunks = []
    stderr_chunks = []
    live_filter = LiveFilter(stream_to_stdout=stream_output)

    sel = selectors.DefaultSelector()
    if proc.stdout:
        sel.register(proc.stdout, selectors.EVENT_READ, "stdout")
    if proc.stderr:
        sel.register(proc.stderr, selectors.EVENT_READ, "stderr")

    active_pipes = (1 if proc.stdout else 0) + (1 if proc.stderr else 0)
    while active_pipes > 0:
        for key, _ in sel.select():
            data = os.read(key.fileobj.fileno(), 4096)
            if not data:
                sel.unregister(key.fileobj)
                active_pipes -= 1
                continue
            if key.data == "stdout":
                text = stdout_decoder.decode(data)
                stdout_chunks.append(text)
                live_filter.feed(text)
            else:
                text = stderr_decoder.decode(data)
                stderr_chunks.append(text)

    remaining_stdout = stdout_decoder.decode(b"", final=True)
    if remaining_stdout:
        stdout_chunks.append(remaining_stdout)
        live_filter.feed(remaining_stdout)
    live_filter.finish()

    remaining_stderr = stderr_decoder.decode(b"", final=True)
    if remaining_stderr:
        stderr_chunks.append(remaining_stderr)

    proc.wait()
    sel.close()

    raw_stdout = "".join(stdout_chunks)
    raw_stderr = "".join(stderr_chunks)

    if proc.returncode != 0:
        err = f"Simulation failed with exit code {proc.returncode}.\n"
        if raw_stderr:
            err += f"Stderr:\n{raw_stderr}\n"
        if raw_stdout:
            err += f"Stdout:\n{raw_stdout}\n"
        raise ToolchainError(err)

    cycles = 0
    cycle_match = re.search(r"Cycles executed before ebreak:\s+(\d+)", raw_stdout)
    if cycle_match:
        cycles = int(cycle_match.group(1))

    clean_output = re.sub(r"Cycles executed before ebreak:\s+\d+\n?", "", raw_stdout)

    return SimResult(
        stdout=clean_output,
        raw_output=raw_stdout,
        cycles=cycles,
        exit_code=proc.returncode,
        streamed=stream_output,
    )
