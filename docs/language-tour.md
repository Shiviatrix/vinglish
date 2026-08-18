## Syntax Overview

Vinglish uses English-like keywords to keep source code readable without abandoning the conventions of a systems language. A small program looks like this:

```vinglish
use std.io

function main()
returns number
begin
    print("Hello from Vinglish!")
    return 0
end
```

This is roughly equivalent to Rust:

```rust
fn main() -> i32 {
    println!("Hello from Vinglish!");
    0
}
```

and C:

```c
#include <stdio.h>
int main(void) {
    puts("Hello from Vinglish!");
    return 0;
}
```

The Vinglish form favors descriptive keywords such as `function`, `begin`, `return`, and `if`. The parser groups these tokens using precedence and block rules. It treats whitespace as a structural cue in some positions, but the language still relies on explicit keywords for control flow and declarations.

## Type System

The type system uses inference and unification. When a value is used in an expression, the compiler infers its type from the usage. When later expressions constrain the same value, the compiler unifies the constraints. If two constraints conflict, it reports a type mismatch directly. This keeps inference deterministic and gives a readable diagnostic path.

Primitive values include numbers, booleans, and text-like values. Aggregate types include structs and tuples. The compiler also supports generic constructs in places such as collections and thread wrappers. A generic function or type is valid when the same type variable is used consistently across the relevant expressions.

## Control Flow

Conditionals are written with `if` and `then`, with `otherwise` used for the fallback branch:

```vinglish
if total is above 10 then
    print("large")
otherwise
    print("small")
end
```

Loops follow the same pattern. `repeat` and `while` forms are available, and loop variables are typed from the iterable or condition that drives them. Nesting is supported as long as blocks terminate cleanly. This is one reason the explicit `begin ... end` form remains important in user code.

## Functions

Functions are declared with a parameter list and an optional return type:

```vinglish
function add(a: number, b: number) returns number
begin
    return a + b
end
```

Argument passing follows the declared semantics of the value, and borrowing is used in the standard library and other low-level APIs when ownership should not move. The return type is part of the function contract, and a mismatch is reported immediately if the body cannot satisfy that type.

## Memory Safety

Vinglish includes ownership and borrowing in the type and ownership passes. This is especially visible in data structures that accept borrowed values rather than moving the container itself. The standard library uses this pattern where a collection should be mutated without being transferred.

The runtime layer also exposes lower-level allocation primitives for objects such as vectors and maps. These are not a user-facing abstraction for normal application code, but they are necessary for collection implementation and for low-level code that needs precise lifetime management. The boundary between safe high-level code and unsafe runtime code is therefore explicit.

## Structs and Data

Structs are declared with a type and explicit fields:

```vinglish
public type Point
begin
    x: number
    y: number
end

let p be Point { x: 10, y: 20 }
println(p.x)
```

This style is useful for systems code because field names remain explicit and records are easier to read than positional tuples. The compiler checks that each field name is declared and that the initializer matches the field type.
