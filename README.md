# SSEOCB
Software Side Executor Of Compressed Binaries is a project with a purpose to allow
execution of programs which are larger than the target device's Flash memory size.

## Background
Small microcontrollers have just a few kilobytes of memory, which limits how large the
programs flashed onto them can be. To run a bigger program, the chip has to be
upgraded (replaced) by a more expensive one. While this is fine for normal users, it
is much more devastating for companies which operate in millions of such devices.

## Target devices
SSEOCB is designed for RV32I architecture. There are several reasons why it was
chosen:
- RV32 is very simple, with most instructions operating only on registers. Memory is
  accessed only by loads and stores, which reduces the complexity of the system.

- RV32I has 32 registers, and also has a reduced architecture RV32E which is similar
  to RV32I with the only difference being that it has 16 registers. This provides a
  great way to isolate guest code from interpreter code, as both can be compiled for
  RV32E and then one simply "remaps" registers to the higher hextet.

- RV32I contains only about 40 instructions, and the handler for all of them is
  relatively small. Other architectures, which have many more instructions, would
  require a much bigger handler.

## Core idea
SSEOCB is made of an interpreter runtime and a tool which prepares object files for
the interpreter.

The interpreter is a small bare-metal program which executes the guest program
instruction by instruction. It contains handlers for all 40 RV32I instructions and
the decompressor code for decoding the next instruction from the compressed payload.
It is written in Rust and assembly.

Guest code and the interpreter are both built for RV32E, which means each uses
registers x0 through x15. Later, however, all register accesses in the interpreter
code are remapped to the higher part of RV32I's register file: x0 stays x0, x1
becomes x16, x2 becomes x17, ..., x15 becomes x30. This way, guest and interpreter
code are fully isolated and never interfere with each other.

The guest code is linked in a special way which detects all memory accesses to
guest's .text section. The address is modified in a special way which is recognizable
by the interpreter. This way, all loads/stores/jumps can handle interpreter or
external code correctly.

SSEOCB's tool scans the object file for all addressable symbols (for example
callbacks later used like `register_callback(&callback)`). These symbols are then
wrapped in uncompressed trampolines which have static addresses and can be safely
passed to external code.

The entire pipeline is a bit complicated and I will explain it in detail later.

## Structure
SSEOCB consists of several crates in `crates/`:
- `sim/` is a small rv32i simulator.
- `int/` is the interpreter rust and assembly code.
- `tool/` is the host-side tool used to prepare object files.
- `rv32-isa/` contains parsing and encoding functions for RV32I instructions.

The rest of the project is currently WIP and might change.

## PoC
The proof of concept for SSEOCB will be considered ready when the following becomes
true:
1. The system is implemented and can correctly execute compressed binaries using `sim/`
2. An example guest program is provided such that `C + I < O` where `C` is the size of the compressed program, `I` is the size of the interpreter, and `O` is the size of the original, uncompressed guest program.
3. The system has been successfully tested on a real device.

> *Currently, none of the steps have been completed*

## Benchmarks
TODO, on real hardware with concrete examples.

## Future work
1. Remove limitations of the current implementation, such as uncompressed .rodata.
2. Improve the tooling.
3. Investigate compression strategies for different use cases.
4. Investigate how performance can be improved by linking critical code as external
   native code.
5. Support other architectures (extensions of RV32, Arm?)
