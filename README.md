# rCore

This is my implemention of rCore based on [rCore-Tutorial-v3](https://github.com/rcore-os/rCore-Tutorial-v3).

# How to build / run / debug

## Prerequisites

- [Rust](https://rustup.rs) and some tools
  - Nightly toolchain
  - `riscv64-unknown-none-elf` target
  - cargo binuntils
  - llvm-tools-preview
  - rust-src

- qemu
- GDB for `riscv64-unknown-none-elf`
- [just](https://github.com/casey/just)

## How to ...

This project uses `just` to save and run commands. You can run `just --help` for help information and run `just --list` to list all available commands.

```bash
cd rcore-os # Enter the `os` directory
just build # Build and link the kernel
just run # Run the kernel with qemu
just debug # Run the kernel with debug mode, waiting for GDB
just gdb # Connect qemu with GDB
```
