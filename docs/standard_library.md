# The Englist Standard Library (`std`)

<div align="center">
  <img src="../Englist%20Icon.png" alt="Englist Logo" width="100" />
</div>

The Englist standard library serves as a primary demonstration of the language's capability. Rather than relying on a monolithic C runtime environment, the core utilities are implemented natively in `.eng` code, guaranteeing zero abstraction overhead.

## Structural Layout

The standard library is organized within the `std/` directory:

```text
std/
├── collections/   (Vector of T, HashMap of K, V, etc.)
├── string.eng     (String manipulation)
├── io.eng         (Terminal I/O)
├── math.eng       (Math routines)
├── net.eng        (Networking & sockets)
└── runtime.eng    (OS-level allocators and wrappers)
```

## The `std.collections` Module

The collections module supplies foundational generic data structures. Internally, these structures utilize raw heap allocation mechanisms, yet they expose a memory-safe, idiomatic Englist API to the user.

### Generic Vector

The `Vector of T` struct provides a dynamically resizing array implementation.

```englist
use std.collections.vector

public function main()
begin
    let v be vector_new of number()
    
    // The vector automatically resizes as elements are appended
    let v be vector_push(v, 10)
    let v be vector_push(v, 20)
    
    // The underlying memory allocation must be explicitly freed
    vector_free(v)
end
```

The runtime required to support these native abstractions is exceptionally minimal, primarily consisting of standard allocation primitives (`malloc`, `free`, `realloc`) defined within `rt.c`.
