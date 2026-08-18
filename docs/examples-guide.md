## Example Repository Layout

The examples directory is grouped by purpose. `basics/` contains the smallest programs; `advanced/` contains more involved logic; `ui_games/` contains graphical demos. The ordering matters because a user should start from syntax and finish with specialized runtime behavior.

## Basics

The first example is `examples/basics/hello.ving`:

```vinglish
use std.io

function main()
returns number
begin
    print("Hello from Vinglish!")
    return 0
end
```

This should be the first file compiled. It shows the function shape, the block form, and the use of `print` and `return`.

## Variables and Arithmetic

A small arithmetic example shows how values are bound and combined:

```vinglish
use std.io

function main()
returns number
begin
    let a be 10
    let b be 5
    let total be a + b
    println(total)
    return 0
end
```

The compiler checks numeric operations on the inferred type. Mismatches are reported before code generation, which keeps arithmetic errors early and explicit.

## Control Flow and Functions

The examples also show conditionals, loops, and recursion. A simple recursive function demonstrates how the type checker resolves calls and return values across multiple frames.

```vinglish
function factorial(n: number) returns number
begin
    if n is below 2 then
        return 1
    end
    return n * factorial(n - 1)
end
```

The language is readable without sacrificing explicit logic. This is one reason the syntax favors descriptive keywords over dense operator-only expressions.

## Structs and Data

Structs are a natural fit for small record data:

```vinglish
public type Point
begin
    x: number
    y: number
end

function main()
returns number
begin
    let p be Point { x: 10, y: 20 }
    println(p.x)
    return 0
end
```

This pattern is common in systems code because it keeps field names visible and reduces confusion when several values are packed into a single record.

## Running Examples

The general workflow is:

```sh
vng check examples/basics/hello.ving
vng build examples/basics/hello.ving --output hello
./hello
```

A good learning path is to begin with `hello.ving`, move to arithmetic or data examples, then continue into algorithmic programs and UI or networking examples. That sequence keeps the syntax and compiler behavior understandable while revealing the capabilities of the standard library in a controlled order.
