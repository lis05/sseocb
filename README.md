# SSEOCB

**Software-Side Executor of Compressed Binaries**

SSEOCB runs programs that are too large for a microcontroller's Flash. It
stores the program compressed and executes it instruction by instruction
through a small interpreter. It needs no MMU or XIP hardware, and it never
decompresses the whole image into RAM.

> [!WARNING]
> **Early stage / work in progress.** The architecture is designed (see
> [`sseocbv3.txt`](sseocbv3.txt)), but only parts of the toolchain exist so far.
> The interpreter has not been implemented yet. See [Status](#status).

---

## Motivation

Some programs are too big for the chips they need to run on. For example, a
small TLS 1.3 client (e.g. mbedTLS, ~46 KiB) cannot run on a CH32V203F6P6,
which has 32 KiB of Flash and 10 KiB of SRAM. Compressing the binary doesn't
help by itself, because there is no room to decompress it. Sliding-window
decompressors also need a large window to get a good ratio, and that window
doesn't fit in the chip's RAM.

SSEOCB takes a different approach:

1. Code is compressed offline.
2. A small interpreter is flashed next to the compressed payload.
3. At runtime, instructions are fetched from the compressed payload with a
   random-access, O(1)-memory decompressor (e.g.
   [blaris](https://github.com/lis05/blaris),
   [graemi](https://github.com/lis05/graemi)) and interpreted one at a time.

## Key Ideas

* **RV32E guest on an RV32I host.** Guest code is compiled for RV32E, so it
  only uses registers `x0`–`x15`. The physical core is RV32I, and the
  interpreter keeps its own state in `x16`–`x31`. Guest registers are the real
  hardware registers: nothing is emulated, and nothing is spilled to RAM.

* **Physical data, virtual code.** Only code lives in the virtual, compressed
  region (vP). Data, `.bss`, heap and stack stay at their normal physical
  addresses. Pointers to data, peripherals and DMA buffers work unchanged, and
  guest code can call native code and be called from it.

* **PLT stubs for address-taken functions.** Native code cannot jump into vP
  directly. Before linking, `tool pre` scans relocations to find every
  function whose address is taken (function pointers, callbacks, tables). It
  generates a small physical stub for each one and redirects the symbol to that
  stub with a linker script. Direct calls between guest functions stay inside
  vP, while taking a function's address yields a real physical address that
  native code and ISRs can call.

The full design (notation, register conventions, instruction handling,
trampolines, toolchain pipeline) is in [`sseocbv3.txt`](sseocbv3.txt). Earlier
iterations are in [`sseocbv1.txt`](sseocbv1.txt) and
[`sseocbv2.txt`](sseocbv2.txt).

---

## Status

| Component | State |
| :--- | :--- |
| `crates/sim`: RV32I instruction-level simulator (memory bus, debugger, E2E tests) | ✅ Working |
| `tool pre`: relocation scan, PLT stubs, linker scripts | ✅ Working |
| `tool post`: extract vP from the linked image, emit PLT table | 🚧 Stub |
| Compression of vP | 🚧 Planned (blaris / graemi first, custom compressor later) |
| Interpreter runtime | 🚧 Planned |
| Virtualizing `.rodata` (currently only `.text` goes into vP) | 🚧 Planned |

---

## Workspace Structure

* **`crates/tool/`**: build-time tool for guest objects.
  * `tool pre <input.o> <output_dir> [--vpbase <addr>] [--vpsize <size>]`
    scans `R_RISCV_32`, `R_RISCV_32_PCREL`, `R_RISCV_HI20`,
    `R_RISCV_PCREL_HI20` and `R_RISCV_PLT32` relocations for address-taken
    functions. It writes the following to the output directory:
    * `trampolines.s`: one PLT stub per address-taken function
    * `plt.ld`: points each such symbol at its stub
    * `virtual.ld`: places guest code in the vP region (default base
      `0xFFF00000`, size 1 MiB)
    * `plt_index.json`: maps each symbol to its slot number
  * `tool post <linked.o> <output_dir>`: *(stub)* will extract the vP
    section and generate the PLT target table.
* **`crates/sim/`**: RV32I simulator used as the test platform. It includes a
  configurable memory bus, an interactive debugger (`sim debug`) and bare-metal
  C end-to-end tests.
* **`playground/`**: a minimal guest program for trying out `tool`.
* **`scripts/`**: helpers for building bare-metal RV32 binaries and running the
  E2E tests.

---

## Building and Testing

Requirements:

* Rust (2024 edition)
* LLVM toolchain with RISC-V support: `clang`, `ld.lld`, `llvm-objcopy`

```bash
# Formatting, clippy, unit tests and bare-metal E2E tests
./test.sh
```

---

## License

This project is dual-licensed under either of:

* **Apache License, Version 2.0** ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
* **MIT License** ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this project by you, as defined in the Apache-2.0 license,
shall be dual-licensed as above, without any additional terms or conditions.
