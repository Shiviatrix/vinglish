# Installation and Verification

Vinglish is built as a Rust workspace and is exposed as the `vng` command. The standard install path is from the repository root:

```sh
cargo install --path crates/vinglish
vng --help
```

The first check is that the binary starts and prints the CLI usage. After installation, run:

```sh
vng --version
```

The output usually begins with the executable name and a semver value such as `0.2.1`. Version numbers matter because they communicate compatibility. In a language project, a stable `x.y.z` series indicates that interface changes are expected to be deliberate and that package constraints should be treated as meaningful. At this stage, Vinglish still has experimental parts, so version values should be read as a signal of maturity rather than a guarantee of full ecosystem stability.

## Project Initialization

A Vinglish project starts with a manifest. Run:

```sh
mkdir demo-app
cd demo-app
vng pkg init
```

This creates `ving.toml` and a `ving.lock` file. The manifest contains the package name, version, and dependency declarations. `package.name` identifies the public package name. `package.version` follows semantic versioning and is used when comparing dependency requirements. The `[dependencies]` section declares the packages the project needs and allows exact versions, ranges, Git URLs, or local paths. The lock file records the exact resolved versions so repeated builds follow the same dependency graph.

The lock file is critical because it prevents silent dependency drift. When a package version changes upstream, the build is still pinned to the previously resolved version until the lock file is intentionally updated.

## First Compilation

The repository includes a simple public example in `examples/basics/hello.ving`:

```vinglish
use std.io

function main()
returns number
begin
    print("Hello from Vinglish!")
    return 0
end
```

Run:

```sh
vng check examples/basics/hello.ving
```

`vng check` parses the file and performs type checking without generating a final binary. It is the fastest way to catch syntax errors, invalid symbols, and wrong types. A successful check produces no error output. To build and run the example:

```sh
vng build examples/basics/hello.ving --output hello
./hello
```

The expected output is:

```text
Hello from Vinglish!
```

The compiler lowers the source through parsing, type inference, IR construction, and backend generation. A failure may come from the front end, the type checker, or the native code generator, so the first diagnostic line is usually the place to start.

## Debugging Compilation Failures

The most common errors are type mismatches and undefined symbols. A type mismatch usually means that a value is inferred as one type and then used where another type is expected. The diagnostic output names the expected type and the actual expression, and it points to the wrong line. The fix is usually straightforward: a function signature, a variable binding, or a parameter type must be corrected.

Undefined symbols normally mean the name is missing or the import path is wrong. If a symbol is not declared in scope or a package dependency is absent, the compiler will report the unresolved name. The error is usually solved by adding the correct `use` statement or by updating `ving.toml`.

C compiler errors are a second layer of failure. After Vinglish lowers to generated C or LLVM code, the native toolchain may fail on ABI or runtime mismatches. In that case the issue is lower-level than parsing. If supported, run the compiler with verbose output to see the stage transitions and narrow the failure to the right phase.

## Development Workflow

### Using the Language Server

Vinglish includes a language server protocol (LSP) implementation that provides editor integration for features like go-to-definition, hover tooltips, and inline diagnostics. To enable it in your editor:

- In VS Code, install the "Vinglish" extension from the marketplace.
- In Neovim, use the built-in LSP client with the `vng-lsp` binary.
- In other editors, point the LSP client to the `vng-lsp` executable located in `target/debug/` after building with `cargo build -p vinglish-lsp`.

The language server is automatically started when you open a `.ving` file in a supported editor. It provides real-time feedback as you type, helping catch errors before you even run `vng check`.

### Formatting Code

Vinglish includes a code formatter, `vng fmt`, that enforces a consistent code style across your project. Run:

```sh
vng fmt src/**/*.ving
```

This will reformat all Vinglish source files in the `src` directory according to the project's style rules. The formatter is opinionated and aims to reduce bikeshedding by enforcing a single, consistent style.

You can integrate the formatter into your editor's save hook or run it as part of a pre-commit hook to ensure all committed code is properly formatted.

### Running Tests

Vinglish projects can include unit tests written in the same language. Tests are defined using the `test` attribute on functions:

```vinglish
#[test]
function test_addition()
returns number
begin
    let result be add(2, 3)
    assert result == 5
    return 0
end
```

To run tests, use:

```sh
vng test
```

This will compile and execute all test functions in the project, reporting any failures. Tests are built with the same optimizations as regular builds, so they reflect the actual performance characteristics of your code.

### Cross-Compilation

Vinglish supports cross-compilation to different target platforms through its C and LLVM backends. To build for a different target, specify the target triple:

```sh
vng build src/main.ving --target x86_64-unknown-linux-gnu --output myapp
```

The available targets depend on what your Rust toolchain supports. You can list available targets with `rustup target list`.

When cross-compiling, ensure that any dependencies (such as linked C libraries) are available for the target platform. The Vinglish compiler will invoke the appropriate linker for the target triple.

## Environment Setup

### Prerequisites

To build Vinglish from source, you need:

- Rust toolchain version 1.70 or newer (install via `rustup`)
- A C compiler (such as gcc or clang) for the C backend
- LLVM development libraries (optional, for the LLVM backend)
- Git (for fetching dependencies from repositories)

On Ubuntu/Debian, you can install the prerequisites with:

```sh
sudo apt-get install build-essential clang llvm-dev libclang-dev
```

On macOS with Homebrew:

```sh
brew install llvm
```

### Building from Source

To build the entire Vinglish toolchain from source:

```sh
git clone https://github.com/username/vinglish.git
cd vinglish
cargo build --release
```

This will produce release binaries in `target/release/`, including `vng`, `vng-lsp`, and `vng-fmt`.

You can then install them system-wide:

```sh
sudo cp target/release/vng /usr/local/bin/
sudo cp target/release/vng-lsp /usr/local/bin/
sudo cp target/release/vng-fmt /usr/local/bin/
```

### IDE Integration

#### Visual Studio Code

1. Install the "Vinglish" extension from the VS Code marketplace.
2. The extension automatically detects the `vng-lsp` binary in your PATH or uses the bundled version.
3. Features provided:
   - Syntax highlighting
   - Go-to-definition (F12)
   - Hover tooltips showing types and documentation
   - Inline errors and warnings
   - Code formatting on save
   - Outline view of symbols

#### Neovim

Using the built-in LSP client:

```lua
-- In your init.lua
local lspconfig = require('lspconfig')
lspconfig.vng_lsp.setup{
    cmd = { "vng-lsp" },
    filetypes = { "ving" },
}
```

#### Emacs

Use `lsp-mode` or `eglot`:

```elisp
(require 'lsp)
(lsp-register-client
 (make-lsp-client
  :new-connection (lsp-stdio-connection "vng-lsp")
  :major-modes #'ving-mode
  :language-id "ving"))
```

Assuming you have a `ving-mode` for syntax highlighting.

## Troubleshooting Installation

### Common Issues

#### "command not found: vng"

If the `vng` command is not found after installation, check that the installation directory is in your PATH. For `cargo install`, the binaries are placed in `$HOME/.cargo/bin` by default. Add this to your shell profile:

```sh
export PATH="$HOME/.cargo/bin:$PATH"
```

#### Permission Errors

If you encounter permission errors when trying to install to system directories, use `cargo install` without sudo (it installs to your user directory) or specify a different installation root with `--root`.

#### Missing Dependencies

If the build fails due to missing linker or library errors, ensure you have the necessary development packages installed. For the C backend, you need a working C compiler and standard library. For the LLVM backend, you need LLVM development libraries matching the version expected by the Rust LLVM bindings.

#### Version Mismatch

If you encounter errors about incompatible versions, check that you are using a compatible Rust toolchain. Vinglish requires Rust 1.70 or newer. You can check your version with `rustc --version` and update with `rustup update`.

## Next Steps

After the first compile succeeds, read the language tour and the standard library reference, then work through the examples in `examples/`. The repository is organized so learning moves from basic syntax to package management, then to broader runtime and compiler behavior. The next documents in this directory cover the syntax, the backend pipeline, and the dependency system in a more concrete way.

For contributors interested in working on the compiler itself, see the [architecture documentation](../explanation/architecture.html) and the [contributor guide](../explanation/contributing.md) (if available). The compiler is organized into clearly separated stages, making it approachable to contribute to specific parts such as the lexer, parser, or code generation backends.