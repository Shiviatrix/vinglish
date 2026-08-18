<div align="center">
  <img src="logos/vinglish-icon-color.svg" alt="Vinglish Icon" width="128" height="128" />
</div>

# Vinglish

Vinglish is an English-inspired systems programming language with a Rust implementation and a compiler pipeline that lowers source code through parser, HIR, MIR, SSA, and backend stages.

## What is in this repo

This repository contains the compiler, runtime, standard library, examples, and public validation flow for Vinglish.

## Install

Requirements:
- Rust 1.96+ (2024 edition)
- A C compiler such as GCC or Clang

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

## Package ecosystem and registry support

The repo ships with a public-first package workflow:

```sh
vng pkg init
vng pkg add core
```

This creates a `ving.toml` manifest and a reproducible `ving.lock` file for dependency tracking. Local registry index lookups and Git/path dependencies are already supported via environment variables and package metadata.

A reference registry index is included at `registry/index.json` and can be used like this:

```sh
VINGLISH_REGISTRY_INDEX=./registry/index.json vng pkg add core
```

This gives the language a realistic dependency story, which is one of the biggest gaps between a research compiler and a world-class language platform.

## Public validation and release expectations

The project now includes a public validation path that is exercised in CI:

```sh
./tests/run_public_checks.sh
```

This script checks:
- the workspace builds cleanly
- the compiler accepts a public example
- a compiled binary runs successfully

## Repository structure

- `crates/` — compiler and runtime crates
- `std/` — public standard library modules
- `examples/` — end-user examples
- `docs/` — architecture and language documentation
- `.github/workflows/` — CI and release automation
- `tests/` — canonical public validation shell checks

## Known status

Vinglish is still experimental, but the public-facing toolchain and validation flow are now structured to support a more reliable release process.

## License

MIT license
