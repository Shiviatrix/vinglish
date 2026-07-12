# The Englist Standard Library (`std`)

<div align="center">
  <img src="../Englist%20Icon.png" alt="Englist Logo" width="100" />
</div>

The Englist standard library serves as a primary demonstration of the language's capability. Rather than relying on a monolithic C runtime environment, the core utilities are implemented natively in `.eng` code, guaranteeing zero abstraction overhead.

## Structural Layout

The standard library is organized within the `std/` directory:

```text
std/
├── collections/   (Vector<T>, HashMap<K,V>, HashSet<T>, etc.)
├── string/        (String manipulation)
├── io/            (Files and terminal IO)
├── fs/            (Filesystem interaction)
├── math/          (Math routines)
├── net/           (Networking)
├── process/       (Subprocesses)
├── sync/          (Concurrency primitives)
├── thread/        (Threading)
├── random/        (RNG)
├── json/          (Data serialization)
├── csv/           (Data parsing)
├── time/          (Time and timers)
└── crypto/        (Cryptography)
```

## The `std.collections` Module

The collections module supplies foundational generic data structures. Internally, these structures utilize raw heap allocation mechanisms, yet they expose a memory-safe, idiomatic Englist API to the user.

### Generic Vector

The `Vector<T>` struct provides a dynamically resizing array implementation.

```englist
use std.collections.vector;

fn main() {
    let v: Vector<number> = vector_new<number>();
    
    // The vector automatically resizes as elements are appended
    v = vector_push(v, 10);
    v = vector_push(v, 20);
    
    // The underlying memory allocation must be explicitly freed
    vector_free(v);
}
```

The runtime required to support these native abstractions is exceptionally minimal, primarily consisting of standard allocation primitives (`malloc`, `free`, `realloc`) defined within `rt.c`.
