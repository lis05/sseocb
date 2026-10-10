# SSEOCB
Software Side Executor Of Compressed Binaries is a project with a purposeSSEOCB is to allow execution of programs which are larger than the target device's
Flash memory size.

## Background
Small microcontrolles have just a few kilobytes of memory, which limits how large the
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
