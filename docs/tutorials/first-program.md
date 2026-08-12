# First Program

This tutorial walks through writing, compiling, and running a Vinglish program.

---

## Prerequisites

- Rust toolchain (edition 2024).
- A C compiler (`cc`, `gcc`, or `clang`).
- Clone the repository and build:

```sh
cargo build --release
```

---

## Step 1: Create a Source File

Create `hello.ving`:

```
function main() returns number
begin
    println("Hello, World!")
    return 0
end
```

This defines a `main` function that prints a string and returns 0.

---

## Step 2: Run with the Interpreter

```sh
vng run hello.ving
```

Expected output:

```
Hello, World!
```

---

## Step 3: Compile to a Native Binary

```sh
vng build hello.ving --output hello
```

This generates `hello.c` and compiles it to a native binary `hello`.

Run the binary:

```sh
./hello
```

---

## Step 4: Type-Check

```sh
vng check hello.ving
```

This runs name resolution, type inference, and ownership checking without producing output.

---

## Next Steps

- See [How-To: Build and Run](../how-to/build-and-run.md) for more build options.
- See [Reference: CLI](../reference/cli.md) for all available commands.
