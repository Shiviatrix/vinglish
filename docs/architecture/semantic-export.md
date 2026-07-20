# Vinglish Semantic Export Contract

## Purpose

The Vinglish compiler exports a stable semantic interchange document for
external tooling. The document is not compiler HIR. HIR remains an internal
representation, free to change without breaking consumers of this contract.

The boundary is intentionally one-way:

```text
source -> lexer -> parser -> HIR -> export builder -> JSON document
```

External tools consume only the JSON document. The compiler has no dependency
on those tools, and the document does not contain compiler IDs, symbol tables,
or HIR variant names.

## Versioning

Every document has these required top-level fields:

```json
{
  "format": "vinglish.semantic-export",
  "version": 1,
  "program": { "modules": [] }
}
```

`format` identifies the contract family. `version` is a whole-number schema
version. Consumers must reject unknown versions rather than guessing at their
meaning. Version 1 is additive only within optional fields; incompatible
changes require a new version.

## Version 1 Vocabulary

Version 1 describes only semantic concepts needed by the first external
reasoning integration:

- program and modules
- functions and parameters
- variables, assignments, and mutations
- calls and returns
- while loops and conditionals
- semantic type descriptions

Expressions retain only identifiers, literals, calls, binary and unary
operations, collections, and an `unsupported` marker. The marker retains a
source range without exposing a compiler-internal construct.

## CLI

The compiler emits the document on standard output:

```bash
vng --emit-ir examples/fibonacci.ving > fibonacci.vinglish-export.json
```

Standard output is reserved for JSON so another tool can consume it directly.
Compiler diagnostics remain on standard error.

## Ownership

The `vinglish-ir-export` crate owns the schema and its HIR-to-transport
builder. It may depend on compiler crates. No external consumer should depend
on this crate; consumers implement the documented versioned JSON contract.
