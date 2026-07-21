# The Vinglish Standard Library

The Vinglish standard library (`std`) provides a focused native layer for
console I/O, allocation-backed collections, strings, files, networking, threads,
and a desktop UI bridge. Modules are written in `.ving` where practical and call
a minimal runtime only at the operating-system boundary.

This gives applications a conventional systems-library model: explicit
resources, statically checked calls, and no hidden allocation or dispatch layer.

## Importing a module

Use the module's dotted path at the top of a `.ving` file:

```vinglish
use std.io
use std.math
use std.collections.vector
```

The compiler resolves `use` declarations through its module graph. Public
declarations are available to importing modules; implementation details remain
local to their source file.

## Module map

```text
std/
├── collections/
│   ├── vector.ving       Generic dynamic storage
│   └── map.ving          Text-to-number map operations
├── file.ving             File and directory helpers
├── io.ving               Console and text helpers
├── math.ving             Numeric routines
├── net.ving              TCP socket wrapper
├── runtime.ving          Allocation and runtime boundary
├── string.ving           String operations
├── subprocess.ving       Process handles and results
├── term.ving             Terminal dimensions
├── thread.ving           Threading primitives
└── ui.ving               Native desktop window and buffer API
```

## Collections

### `Vector<T>`: generic dynamic storage

`Vector<T>` is the generic collection often described as a **Vector of T**. Its
source spelling makes the element type explicit; its purpose is an ordered,
dynamically sized sequence of values of type `T`.

```vinglish
use std.collections.vector
use std.io

function main()
begin
    let scores be vector_new<number>()
    push(borrow mutable scores, 42)
    push(borrow mutable scores, 99)

    println(get(borrow scores, 1))
    vector_free(borrow mutable scores)
end
```

`vector_new<T>()` creates a vector, `push` appends through a mutable borrow, and
`get` reads an indexed value through a read-only borrow. The current
implementation stores allocation metadata in this explicit record:

```vinglish
public type Vector<T>
begin
    data: address<number>
    len: number
    capacity: number
end
```

The vector owns its backing allocation. Call `vector_free` exactly once after
the vector is no longer needed, then do not access the released value.

### `Map`: text-keyed values

`std.collections.map` provides a map with `text` keys and `number` values.
Create it with `map_new`, mutate it with `insert`, query it with `get`, and
release it with `map_free`.

```vinglish
use std.collections.map

function main() returns number
begin
    let settings be map_new()
    insert(borrow settings, "workers", 4)
    let workers be get(borrow settings, "workers")
    map_free(borrow settings)
    return workers
end
```

## Input, output, and text

`std.io` contains console output and basic text helpers. Use `print` for output
without a newline and `println` for output with one.

```vinglish
use std.io

function main()
begin
    let user be read_line()
    println(fmt!("Hello, {user}."))
end
```

The module also includes `starts_with`, `substring`, `substring_len`, and
`index_of`. `std.string` supplies allocation-backed string routines including
`string_new`, `length`, `concat`, and `string_free`.

Use the built-in `fmt!` macro whenever a message combines readable values into
a single `text` value:

```vinglish
use std.io
use std.math

function report(number left, number right)
begin
    let larger be max(left, right)
    println(fmt!("The larger value is {larger}."))
end
```

## Files and processes

`std.file` exposes direct helpers for filesystem work:

- `file_read(path: string) returns string`
- `file_write(path: string, content: string)`
- `dir_list(path: string) returns string`

`std.subprocess` defines `Process` and `ProcessResult` types for runtime-backed
process integration. Validate external input, surface recoverable failure with
`Result of T` in your own API, and document ownership of every returned handle.

## Networking and `Result of T`

`std.net` wraps TCP sockets in `TcpSocket`. Connection, send, and receive
operations return `Result of T`, making ordinary network failure visible in the
caller's type signature.

```vinglish
use std.net
use std.io

function send_probe() returns Result of number
begin
    let socket be tcp_connect("127.0.0.1", 8080)?
    let written be tcp_send(borrow socket, fmt!("ping"))?
    tcp_close(borrow socket)
    return Ok(written)
end
```

The postfix `?` operator propagates an error without turning a recoverable
network failure into an exception. See
[Errors with `Result of T`](language_guide.md#errors-with-result-of-t) for the
complete language model.

## Native UI

`std.ui` is a native desktop API backed by Vinglish's Rust runtime bridge. It
supplies window, pixel-buffer, and update primitives for small desktop tools and
visual experiments.

Use `is_space_pressed(window)` to read the Space key for a created window. The
function is exported by the Rust UI runtime and is useful for simple interactive
tools and games.

```vinglish
use std.ui

function main() returns number
begin
    let width be 640
    let height be 400
    let window be create_window("Vinglish", width, height)
    let buffer be create_buffer(width, height)

    if is_window_open(window) is 0 then begin
        return 1
    end

    fill_buffer(buffer, 1973790)
    if is_space_pressed(window) is 1 then begin
        set_pixel(buffer, 40, 20, 15247447)
    end
    set_pixel(buffer, 20, 20, 15247447)
    update_window(window, buffer)
    return 0
end
```

The UI boundary calls Rust functions exported through `#[vinglish_export]`.
Vinglish sees ordinary foreign declarations; the macro generates the
C-compatible bridge for supported Rust functions.

See [Skyline Runner](playground.md) for a complete native UI example and its
interactive browser preview.

## Runtime boundary and library guidance

`std.runtime` owns low-level allocation and memory primitives used by library
implementations. Prefer higher-level APIs such as `Vector<T>` in applications.
When building a low-level abstraction, document its allocation, borrowing, and
release rules as part of the public API.

- Use `public type` with `name: type` fields for public data structures.
- Keep generic containers explicit: `Vector<T>` in code, “Vector of T” in prose
  where it improves readability.
- Return `Result of T` for operations that can fail in normal use.
- Accept `borrow` or `borrow mutable` when an API reads or updates caller-owned
  data in place.
- Pair resource creation with a documented release operation.
