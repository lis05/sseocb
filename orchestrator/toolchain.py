import os
from pathlib import Path
import shlex
import shutil
import subprocess
from typing import Dict, List, Optional, Union


class ToolchainError(RuntimeError):
    pass


class Toolchain:
    def __init__(
        self,
        repo_root: Optional[Path] = None,
        log: bool = False,
        build_crates: bool = True,
    ):
        if repo_root is None:
            repo_root = Path(__file__).resolve().parent.parent
        self.repo_root = repo_root
        self.log = log

        self.clang = self._find_tool("clang")
        self.ld_lld = self._find_tool("ld.lld")
        self.llvm_objcopy = self._find_tool("llvm-objcopy")
        self.llvm_readelf = self._find_tool("llvm-readelf")
        self.cargo = self._find_tool("cargo")

        if build_crates:
            self.build_host_crates()

        self.sim = self._get_target_bin("sim")
        self.tool = self._get_target_bin("tool")

    def _find_tool(self, name: str) -> Path:
        found = shutil.which(name)
        if not found:
            raise ToolchainError(f"Required tool '{name}' not found in PATH")
        return Path(found)

    def build_host_crates(self) -> None:
        """Build crates/tool, crates/rv32-isa, and crates/sim before running anything."""
        self.run(
            [
                self.cargo,
                "build",
                "--manifest-path",
                str(self.repo_root / "Cargo.toml"),
                "-p",
                "tool",
                "-p",
                "rv32-isa",
                "-p",
                "sim",
            ],
            cwd=self.repo_root,
        )

    def _get_target_bin(self, crate_name: str) -> Path:
        bin_path = self.repo_root / "target" / "debug" / crate_name
        if not bin_path.exists():
            self.build_host_crates()
        if not bin_path.exists():
            raise ToolchainError(f"Failed to find or build '{crate_name}' at {bin_path}")
        return bin_path

    def _ensure_target_bin(self, crate_name: str) -> Path:
        return self._get_target_bin(crate_name)

    def ensure_int_lib(self, decompressor: str = "noop") -> Path:
        int_a = (
            self.repo_root
            / "target"
            / "riscv32e-unknown-none-elf"
            / "release"
            / "libint.a"
        )
        stamp_file = int_a.parent / ".decompressor"
        dec_norm = decompressor.lower()
        needs_build = not int_a.exists() or (
            not stamp_file.exists() or stamp_file.read_text().strip() != dec_norm
        )
        if needs_build:
            if dec_norm == "noop":
                feature_args = ["--no-default-features", "--features", "noop-dec"]
            elif dec_norm == "blaris":
                feature_args = ["--no-default-features", "--features", "blaris-dec"]
            else:
                feature_args = ["--no-default-features", "--features", f"{dec_norm}-dec"]

            env = os.environ.copy()
            existing_rustflags = env.get("RUSTFLAGS", "")
            env["RUSTFLAGS"] = f"{existing_rustflags} -Zunstable-options -Cpanic=immediate-abort".strip()

            self.run([
                self.cargo,
                "+nightly",
                "build",
                "-p",
                "int",
                "--release",
                "--target",
                "riscv32e-unknown-none-elf",
                "-Z",
                "build-std=core,compiler_builtins",
                *feature_args,
            ], cwd=self.repo_root, env=env)
            int_a.parent.mkdir(parents=True, exist_ok=True)
            stamp_file.write_text(dec_norm)

        if not int_a.exists():
            raise ToolchainError(f"Failed to find or build 'libint.a' at {int_a}")
        return int_a

    def run(
        self,
        cmd: List[Union[str, Path]],
        cwd: Optional[Path] = None,
        env: Optional[Dict[str, str]] = None,
        capture_output: bool = True,
        check: bool = True,
    ) -> subprocess.CompletedProcess:
        str_cmd = [str(x) for x in cmd]
        if self.log:
            print(f"+ {shlex.join(str_cmd)}", flush=True)
        res = subprocess.run(
            str_cmd,
            cwd=str(cwd) if cwd else None,
            env=env,
            capture_output=capture_output,
            text=True,
        )
        if check and res.returncode != 0:
            err_msg = (
                f"Command failed with exit code {res.returncode}:\n"
                f"Command: {shlex.join(str_cmd)}\n"
            )
            if res.stderr:
                err_msg += f"Stderr:\n{res.stderr}\n"
            if res.stdout:
                err_msg += f"Stdout:\n{res.stdout}\n"
            raise ToolchainError(err_msg)
        return res
