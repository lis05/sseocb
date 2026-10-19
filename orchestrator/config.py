from dataclasses import dataclass, field
from pathlib import Path
from typing import List, Optional


@dataclass
class Config:
    subcommand: str
    target: str

    guest_obj: Path
    memory_ld: Path
    output_dir: Path

    guest_ld: Optional[Path] = None
    guest_linker_flags: List[str] = field(default_factory=list)

    guest_native_obj: Optional[Path] = None
    guest_native_ld: Optional[Path] = None
    guest_native_linker_flags: List[str] = field(default_factory=list)

    include_dirs: List[Path] = field(default_factory=list)
    c_flags: List[str] = field(default_factory=list)

    entry: str = "_start"
    vpbase: int = 0xFFF00000
    vpsize: int = 0x00100000
    decompressor: str = "noop"

    print_output: bool = False
    debug: bool = False
    expected_output: Optional[Path] = None
    max_cycles: int = 0
    log: bool = False
    size: bool = False
    size_only: bool = False
    native_c: bool = True
