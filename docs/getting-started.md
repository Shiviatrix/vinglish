## Installation and Verification

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

## Next Steps

After the first compile succeeds, read the language tour and the standard library reference, then work through the examples in `examples/`. The repository is organized so learning moves from basic syntax to package management, then to broader runtime and compiler behavior. The next documents in this directory cover the syntax, the backend pipeline, and the dependency system in a more concrete way.
