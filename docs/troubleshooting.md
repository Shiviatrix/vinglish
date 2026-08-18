## Compilation Errors

The most common issue is a type mismatch. A message such as `expected address<number>, got ()` indicates that a function call or return path produced the wrong value shape. The fix is usually in the function signature or the variable binding at the reported location. Undefined symbol errors are equally common and usually indicate a missing import or a package not listed in `ving.toml`.

C compiler errors are different. They appear after the compiler has generated C or LLVM code and the native toolchain fails. In that case, the source problem is lower-level than the language front end. The generated output should be treated as a code-generation artifact, and the ABI or runtime boundary should be inspected first.

## Runtime Errors

Runtime failures usually indicate that the program reached a boundary that was not checked during compile time. This is common in pointer-heavy or runtime-backed code, especially around collection operations or low-level memory access. Debugging is usually done by reducing the program to a minimal reproducer and inspecting the failing function.

When the target is a native backend, GDB or LLDB can be helpful for segmentation faults or invalid memory access. A debugger is more informative than the compiler output when the program fails deep in a runtime helper.

## Performance Issues

Most performance problems come from repeated allocation, overly large copies, or unnecessary runtime calls inside loops. The compiler has optimization passes, but they do not replace a better algorithm. A slower path is often the result of allocation churn or repeated collection growth, not just poor code generation.

## Installation Problems

The standard install flow requires a working Rust toolchain and a C compiler. On Linux, GCC or Clang is usually enough. On macOS, Xcode command-line tools provide the required toolchain. A missing compiler or a broken `PATH` setup is the usual root cause of installation problems.

## Package Management Problems

Version conflicts and registry failures are the usual package issues. The package manager checks the manifest requirement against the `ving.lock` version and raises an error when they do not match. When the registry does not respond, the environment variable `VINGLISH_REGISTRY_INDEX` can point to a local or offline index for debugging and validation.

## Getting Help

The best bug report includes the exact command, the failing source file, and the compiler output. If the problem is in the package system, include the manifest and lock file. This gives maintainers a direct reproduction path and reduces the time required to isolate the root cause.
