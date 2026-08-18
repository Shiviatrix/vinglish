<div align="center">
  <img src="logos/vinglish-icon-color.svg" alt="Vinglish Icon" width="128" height="128" />
</div>

# Vinglish

Vinglish is a systems programming language with English-like syntax that compiles through a multi-stage pipeline: lexing, parsing, type inference, and intermediate representations (HIR → MIR → SSA) before lowering to C or LLVM. The compiler is implemented in Rust.

## Contents

This repository contains the compiler toolchain, runtime, standard library, examples, and the validation procedures used in CI.

## Installation

Requirements:
- Rust 1.96+ (2024 edition)
- GCC or Clang

```sh
cargo install --path crates/vinglish
vng --help
```

## Quick start

```sh
vng check examples/basics/hello.ving
vng build examples/basics/hello.ving --output hello
./hello
```

Expected output:
```text
Hello from Vinglish!
```

## Package management

The language includes a package manager with dependency tracking:

```sh
vng pkg init
vng pkg add core
```

This creates a `ving.toml` manifest and `ving.lock` file for reproducible builds. The package manager supports local registry indices, Git-based dependencies, and path-based dependencies through environment variables.

A reference registry index is provided at `registry/index.json`:

```sh
VINGLISH_REGISTRY_INDEX=./registry/index.json vng pkg add core
```

The standard library now also includes richer public modules for common runtime tasks, including `std.time`, `std.random`, and `std.collections.list` for timestamps, RNGs, and resizable generic lists.

The package registry is now semver-aware: dependency ranges are resolved against versioned registry metadata and `ving.lock` records the installed version plus integrity data so lockfile drift is detected immediately.

## Example library

The repository keeps examples organized by use case:

- `examples/basics/` — syntax and compiler basics
- `examples/apps/` — practical end-user programs
- `examples/games/` — lightweight entertainment demos
- `examples/tools/` — automation and utility scripts
- `examples/advanced/` — larger experiments and edge cases

## Validation and testing

The project includes automated validation checks run in CI:

```sh
./tests/run_public_checks.sh
```

These checks verify:
- The workspace builds successfully
- The compiler handles public examples correctly
- Compiled binaries execute as expected

## Repository structure

- `crates/` — compiler and runtime implementation
- `std/` — standard library modules
- `examples/` — usage examples
- `docs/` — architecture and language documentation
- `.github/workflows/` — CI and release automation
- `tests/` — validation procedures

## Status

Vinglish is experimental. The public toolchain and validation framework are structured to support reliable releases.

## License

MIT
