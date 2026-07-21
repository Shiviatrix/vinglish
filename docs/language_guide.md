# Vinglish Language Guide

Vinglish is a statically typed systems programming language that reads like clear
prose while compiling to efficient, low-level C. It combines deterministic data
layout, explicit ownership, and a deliberately small surface language with a
multi-stage optimizing compiler.

This guide documents the implemented language. Read [Architecture](architecture.md)
for the compiler pipeline and [The Standard Library](standard_library.md) for
shipped modules.

## Getting started

Vinglish source files use the `.ving` extension. Use `vng` to check, run, build,
and format a program:

```bash
vng check hello.ving
vng run hello.ving
vng build hello.ving --output hello
vng fmt hello.ving
```

The default build path emits C, links the minimal runtime, and invokes the host
C toolchain. Use `--emit c` to inspect generated C or `--emit mir` to inspect
the optimized mid-level representation.

```vinglish
use std.io

function main()
begin
    println("Hello from Vinglish.")
end
```

## Declarations and functions

Declare a local value with `let ... be ...`. Vinglish infers the type from its
initializer whenever it can do so unambiguously.

```vinglish
function add(number left, number right) returns number
begin
    let total be left + right
    return total
end
```

Parameters support both the prose-oriented `type name` form and the name-first
`name: type` form. Use one convention consistently within a public API.

```vinglish
function scale(value: number, factor: number) returns number
begin
    return value * factor
end
```

Export library declarations with `public`. Symbols supplied by a runtime or
foreign library use `foreign function`.

```vinglish
public foreign function monotonic_time() returns number

public function elapsed_since(number start) returns number
begin
    return monotonic_time() - start
end
```

## Types and records

Record types are declared with `type`; `struct` is not a Vinglish keyword.
Fields use `name: type`, and values are created with brace literals.

```vinglish
type Point
begin
    x: number
    y: number
end

function origin() returns Point
begin
    return Point { x: 0, y: 0 }
end
```

Field access uses dot notation. An explicit record definition maps naturally to
the generated low-level representation.

```vinglish
function distance_from_origin(Point point) returns number
begin
    return point.x * point.x + point.y * point.y
end
```

The core scalar vocabulary includes `number`, `decimal`, `text`, `boolean`, and
`address<T>`. Named `type` declarations and generic instantiations are checked
statically.

## Methods and ownership

Methods are extension methods declared with `on`. They receive an implicit
`self` value and are called with dot syntax. Do not model a method as a standard
function that accepts its record as the first argument.

```vinglish
type Point
begin
    x: number
    y: number
end

function translate on Point (number dx, number dy) returns Point
begin
    return Point {
        x: self.x + dx,
        y: self.y + dy
    }
end

function main()
begin
    let start be Point { x: 10, y: 20 }
    let destination be start.translate(5, -3)
    println(destination.x)
end
```

Use `borrow` for a read-only reference and `borrow mutable` for a reference a
callee may update in place.

```vinglish
type Counter
begin
    value: number
end

function increment on Counter ()
begin
    self.value be self.value + 1
end
```

Prefer returning a replacement value for small, immutable transformations.
Reserve mutable borrows for APIs that genuinely update caller-owned state. The
compiler validates ownership, borrowing, and aliasing before code generation.

## Control flow

Blocks begin with `begin` and end with `end`. Conditions read naturally with
operators such as `is`, `is below`, and ordinary arithmetic comparisons.

```vinglish
function classify(number value) returns text
begin
    if value is below 0 then begin
        return "negative"
    end

    if value is 0 then begin
        return "zero"
    end

    return "positive"
end
```

Use `repeat while` for condition-driven iteration:

```vinglish
function sum_to(number limit) returns number
begin
    let total be 0
    let current be 1

    repeat while current <= limit
    begin
        total be total + current
        current be current + 1
    end

    return total
end
```

`match` selects among literal cases and supports a final `otherwise` branch.

```vinglish
function status_label(number code) returns text
begin
    match code
        case 200 then return "ok"
        case 404 then return "not found"
        otherwise then return "unexpected"
end
```

## Generics and collections

Generic types use angle brackets in source. In prose, a **Vector of T** means a
vector parameterized by element type `T`; in a declaration it is written
`Vector<T>`. This keeps the explanation English-like without obscuring the
concrete type argument from the compiler.

```vinglish
type Pair<T>
begin
    first: T
    second: T
end

function first_of<T>(Pair<T> pair) returns T
begin
    return pair.first
end
```

The standard library's `Vector<T>` is a generic, dynamically sized collection.
Create it with `vector_new<T>()`, append through a mutable borrow, and release
its backing allocation when it is no longer needed.

```vinglish
use std.collections.vector
use std.io

function main()
begin
    let values be vector_new<number>()
    push(borrow mutable values, 10)
    push(borrow mutable values, 20)

    println(get(borrow values, 0))
    vector_free(borrow mutable values)
end
```

Generic uses are type-checked at each call site and lower without a dynamic
dispatch layer. See [The Standard Library](standard_library.md#collections) for
the available collection modules.

## Errors with `Result of T`

Vinglish represents recoverable failure explicitly with `Result of T`. A
successful operation returns `Ok(value)`; a failed operation returns
`Err(message)`. No implicit exception path crosses a function boundary.

The postfix `?` operator unwraps an `Ok` value or returns the error from the
current `Result of ...` function. It keeps the successful path linear while the
type signature preserves the error path.

```vinglish
use std.net
use std.io

function fetch_local_status() returns Result of number
begin
    let socket be tcp_connect("127.0.0.1", 8080)?
    let request be fmt!("GET /status HTTP/1.1\\r\\nHost: localhost\\r\\n\\r\\n")
    let sent be tcp_send(borrow socket, request)?

    tcp_close(borrow socket)
    return Ok(sent)
end

function main() returns Result of number
begin
    let sent be fetch_local_status()?
    println(fmt!("Sent {sent} bytes."))
    return Ok(0)
end
```

## Text and `fmt!`

`fmt!` is Vinglish's formatting macro. It creates `text` from a format literal
and interpolates identifiers or expressions in braces. Use it for diagnostics,
user-facing messages, and protocol payloads instead of manually concatenating
many fragments.

```vinglish
function greeting(text name, number release) returns text
begin
    return fmt!("Welcome, {name}. Vinglish release {release} is ready.")
end
```

## Modules

Import modules with `use` followed by a dot-separated path:

```vinglish
use std.io
use std.math
use std.collections.vector
```

The compiler resolves imports into a module graph before semantic analysis. A
module is a separate `.ving` file, giving large programs clear namespaces and
enabling incremental, per-module work.

## Practical conventions

- Use the `.ving` extension for all Vinglish source.
- Declare record-like data with `type`, never `struct`.
- Define each field as `name: type` and construct records with braces.
- Use `function name on Type (...)` for behavior attached to a type.
- Return `Result of T` for operations that can fail in normal use.
- Run `vng fmt` and `vng check` before sharing or building a module.
