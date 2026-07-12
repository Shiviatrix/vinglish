# Englist Language Guide

<div align="center">
  <img src="../Englist%20Icon.png" alt="Englist Logo" width="100" />
</div>

Englist is designed to maximize human readability while preserving the low-level performance characteristics of a systems programming language like C. This guide outlines the core syntactical features of the language.

## Variables and Functions

Variables in Englist are strongly typed to ensure program correctness.

```englist
fn add(a: number, b: number) -> number {
    return a + b;
}
```

## Structs and Pointers

Englist provides support for C-style structs. Memory management is explicit, requiring the use of pointers to modify data structures in place.

```englist
struct Point {
    x: number,
    y: number
}

fn move_point(p: *Point, dx: number, dy: number) {
    p->x = p->x + dx;
    p->y = p->y + dy;
}
```

## Generics

Generics are implemented as a first-class feature in Englist. They are heavily utilized throughout the standard library to enable type-safe code reuse without runtime overhead.

```englist
struct Pair<T> {
    first: T,
    second: T
}

fn get_first<T>(p: Pair<T>) -> T {
    return p.first;
}
```

## Modules (Import Graph)

To support scalable software development, Englist resolves dependencies into a Module Graph rather than a monolithic AST. This architectural decision facilitates fast incremental compilation and ensures clean namespace management.

```englist
use std.math;
use std.collections.vector;
```
