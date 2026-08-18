## Organization

The standard library is stored under `std/` and organized by function. Modules such as `std.io`, `std.file`, `std.math`, and `std.collections.map` represent separate responsibilities. The layout keeps low-level runtime helpers separate from user-facing APIs. This matters because runtime memory operations are more fragile than ordinary file or math calls.

## Core Modules

`std.io` provides read and print helpers. `std.math` provides numeric utilities and constants. `std.file` provides file read and write operations. `std.runtime` exposes low-level memory helpers used by the collection framework. These modules are the baseline for everyday Vinglish programs and for the runtime code that supports the language itself.

## String Handling

The string API is defined in `std.string`. It exposes creation, concatenation, length, and cleanup routines. This is the right place to perform string operations instead of writing ad hoc runtime calls in user code. The abstraction keeps string behavior consistent and avoids repeated direct memory handling in application code.

```vinglish
use std.string

function main()
returns number
begin
    let hello be string_new("hello")
    let world be string_new(" world")
    let msg be concat(hello, world)
    println(length(msg))
    string_free(msg)
    return 0
end
```

## I/O and File Access

The basic I/O layer is small and direct. It reads lines from stdin and writes output to stdout. File access is handled separately through `std.file` and is intended for simple read/write workflows rather than a full streaming abstraction.

```vinglish
use std.file

function main()
returns number
begin
    let content be file_read("example.txt")
    println(content)
    return 0
end
```

## Collections

The collection modules include list-like and map-like structures. `std.collections.list` provides a small dynamic list API, and `std.collections.map` provides a key-value container. These modules wrap runtime allocation and index operations, so they are useful but not meant to hide the underlying memory model completely.

```vinglish
use std.collections.list

function main()
returns number
begin
    let xs be list_new()
    push(borrow xs, 10)
    push(borrow xs, 20)
    println(length(borrow xs))
    list_free(borrow xs)
    return 0
end
```

## Error Handling

The public modules often return structured failure results or status values instead of relying on hidden exceptions. This is a deliberate design because systems code benefits from explicit failure paths. This is especially visible in modules that touch networking, processes, and file I/O.

## Memory Utilities

`std.runtime` is the low-level memory layer. It provides allocation, free, and pointer operations used by the collection implementations. This is not the main API for ordinary application code, but it is the correct place for low-level data structures and runtime-backed abstractions.
