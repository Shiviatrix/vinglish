# Standard Library

The Vinglish standard library consists of `.ving` modules in `std/` with corresponding C runtime implementations in `rt/`.

---

## Purpose

Provide built-in functionality for I/O, strings, math, files, networking, threads, subprocesses, terminal interaction, UI, and collections.

---

## Modules

| Module | Path | C Runtime | Description |
|---|---|---|---|
| `std.io` | `std/io.ving` | `rt/ving_io.c` | Line reading, string operations |
| `std.string` | `std/string.ving` | `rt/ving_string.c` | Heap-allocated String type with create/concat/length/free |
| `std.math` | `std/math.ving` | (libm) | Constants (PI, E), trig functions, abs, min, max |
| `std.file` | `std/file.ving` | `rt/ving_file.c` | File read/write |
| `std.net` | `std/net.ving` | `rt/ving_net.c` | TCP connect, send, receive, close |
| `std.thread` | `std/thread.ving` | `rt/ving_thread.c` | Thread type, sleep |
| `std.subprocess` | `std/subprocess.ving` | `rt/ving_subprocess.c` | Process spawning, output capture |
| `std.term` | `std/term.ving` | `rt/ving_term.c` | Raw mode, key reading, cursor control, colors |
| `std.ui` | `std/ui.ving` | `rt/ving_ui.c` (+ `rt_rust`) | Window creation, pixel drawing, event polling |
| `std.runtime` | `std/runtime.ving` | `rt/ving_runtime.c` | Low-level alloc/free |
| `std.collections.vector` | `std/collections/vector.ving` | — | Generic vector (uses `std.runtime`) |
| `std.collections.map` | `std/collections/map.ving` | `rt/ving_map.c` | String-keyed map |

---

## Module Details

### std.io

Public functions:
- `read_line() returns string`
- `starts_with(str, prefix) returns number`
- `substring(str, start) returns string`
- `index_of(str, delimiter) returns number`
- `substring_len(str, start, len) returns string`
- `unescape_newlines(str) returns string`

All delegate to `ving_*` foreign functions implemented in `rt/ving_io.c`.

### std.string

Defines a heap-allocated `String` type with a `ptr: text` field.

Public functions:
- `string_new(text p) returns String`
- `length(borrow String s) returns number`
- `concat(borrow String a, borrow String b) returns String`
- `string_free(borrow String s)`

### std.math

Constants: `PI` (3.141592653589793), `E` (2.718281828459045).

Foreign functions (libm): `pow`, `sin`, `cos`, `tan`, `sqrt`, `log`, `log10`, `exp`, `ceil`, `floor`, `round`.

Vinglish functions: `abs(x)`, `min(a, b)`, `max(a, b)`.

### std.file

Public foreign functions: `ving_file_read(path) returns string`, `ving_file_write(path, content)`.

### std.net

Public foreign functions: `ving_net_tcp_connect(host, port)`, `ving_net_tcp_send(sock, data)`, `ving_net_tcp_recv(sock)`, `ving_net_tcp_close(sock)`.

### std.runtime

Public foreign functions: `ving_alloc(size) returns address<number>`, `ving_free(ptr)`.

---

## Usage

Import a standard library module:
```
use std.io
use std.math
use std.collections.vector
```

The `$VINGLISH_ROOT` environment variable must point to the repository root for `std` resolution. Otherwise, `std/` is resolved relative to the current working directory.

---

## Limitations

- No standard library documentation beyond function signatures.
- The `String` type is separate from the built-in `text` type.
- Collections rely on raw pointer arithmetic via `std.runtime`.

---

## Related Components

- [Reference: CLI](cli.md)
- [Architecture](../explanation/architecture.md)
