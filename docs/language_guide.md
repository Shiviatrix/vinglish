# Englist Language Guide

<div align="center">
  <img src="../Englist%20Icon.png" alt="Englist Logo" width="100" />
</div>

Englist is designed to maximize human readability while preserving the low-level performance characteristics of a systems programming language like C. This guide outlines the core syntactical features of the language.

## Variables and Functions

Variables in Englist are strongly typed to ensure program correctness. Use `let ... be ...` to declare variables.

```englist
public function add(number a, number b) returns number
begin
    return a + b
end
```

## Structs and Borrowing

Englist provides support for structs. Memory management is explicit, requiring the use of `borrow` to modify data structures in place or pass references.

```englist
struct Point begin
    number x
    number y
end

public function move_point(borrow Point p, number dx, number dy)
begin
    let p.x be p.x + dx
    let p.y be p.y + dy
end
```

## Generics

Generics are implemented as a first-class feature in Englist. They are heavily utilized throughout the standard library to enable type-safe code reuse without runtime overhead.

```englist
struct Pair of T begin
    T first
    T second
end

public function get_first of T (Pair of T p) returns T
begin
    return p.first
end
```

## Error Handling

Englist uses a `Result of T` type and the postfix `?` operator for robust and explicit error propagation, completely avoiding exceptions.

```englist
public function read_config() returns Result of text
begin
    let file be open("config.txt")?
    let content be read_to_string(borrow file)?
    return Ok(content)
end
```

## String Formatting

You can easily format strings using the `fmt!` macro, which seamlessly interpolates variables and expressions into text.

```englist
public function greet(text name, number age)
begin
    let message be fmt!("Hello, my name is {name} and I am {age} years old.")
    print(message)
end
```

## Modules (Import Graph)

To support scalable software development, Englist resolves dependencies into a Module Graph rather than a monolithic AST. This architectural decision facilitates fast incremental compilation and ensures clean namespace management.

```englist
use std.math
use std.collections.vector
```
