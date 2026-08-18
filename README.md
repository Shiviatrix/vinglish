# Vinglish

Vinglish is a systems programming language with English-like syntax, ownership-aware checks, and a multi-stage compiler pipeline. It aims to be readable for novices while still supporting low-level runtime and compiler-level work.

## What is in this repo

- `crates/` — compiler, runtime, and standard-library toolchain
- `std/` — public runtime modules
- `examples/` — end-user and teaching examples
- `docs/` — user documentation and compiler reference material
- `registry/` — sample package registry metadata
- `.github/workflows/` — CI and release automation
- `tests/` — validation scripts and smoke checks

## Installation

Requirements:

- Rust toolchain (stable)
- C toolchain (GCC or Clang)

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

## Documentation map

Start here and then move deeper:

- [docs/README.md](docs/README.md)
- [docs/getting-started.md](docs/getting-started.md)
- [docs/language-tour.md](docs/language-tour.md)
- [docs/compiler-pipeline.md](docs/compiler-pipeline.md)
- [docs/standard-library.md](docs/standard-library.md)
- [docs/package-management.md](docs/package-management.md)
- [docs/examples-guide.md](docs/examples-guide.md)
- [docs/troubleshooting.md](docs/troubleshooting.md)

## Package management

```sh
vng pkg init
vng pkg add core
```

The package manager creates a `ving.toml` manifest and a `ving.lock` file, resolves semver-aware versions, and validates dependency integrity before building.

## Validation

The canonical public validation path is:

```sh
./tests/run_public_checks.sh
```

It verifies the workspace builds and that the main public example executes successfully.

## Status

The project is structured for public-facing use, but it still carries an experimental label in some areas. The compiler, runtime, standard library, package manager, and docs are all publicly testable and documented.

## License

MIT
