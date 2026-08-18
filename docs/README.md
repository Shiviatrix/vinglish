# Vinglish documentation

This directory is organized to match the way people actually learn the language: start with installation, then move into the language model, then product tooling and package management, and finally the runtime and compiler internals.

## Start here

- [Getting started](./getting-started.md) — install, verify the CLI, and compile your first program.
- [Language tour](./language-tour.md) — syntax, data model, control flow, and basic semantics.
- [Compiler pipeline](./compiler-pipeline.md) — how source becomes executable output.
- [Standard library](./standard-library.md) — runtime modules and public APIs.
- [Package management](./package-management.md) — manifests, lockfiles, semver, and registries.
- [Examples guide](./examples-guide.md) — practical example progression.
- [Troubleshooting](./troubleshooting.md) — common errors and recovery paths.

## Supporting reference material

The repo also includes deeper design docs and reference pages:

- `docs/reference/` — lexer, parser, ownership, MIR, SSA, and code generation references
- `docs/explanation/` — architecture and compiler rationale
- `docs/tutorials/` — intro tutorials
- `docs/adr/` — architecture decision records

## Recommended reading order

1. Install and verify the toolchain.
2. Read the language tour and the first-program tutorial.
3. Understand the compiler stages.
4. Learn the package ecosystem and standard library.
5. Use the examples to build practical programs.
6. Troubleshoot using the known error patterns.

This structure keeps the public-facing docs clean while still preserving the deeper compiler reference content for contributors and advanced users.
