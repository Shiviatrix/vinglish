# Vinglish Semantic Export Contract

## Purpose

Vinglish can emit a stable semantic interchange document for external tooling.
The document is intentionally **not** compiler HIR: HIR is internal and may
change as the compiler evolves. Consumers integrate with this versioned JSON
contract instead.

The boundary is one-way:

```text
.ving source → lexer → parser → HIR → export builder → JSON document
```

External tools consume JSON output. The compiler does not depend on those tools,
and the document never exposes compiler IDs, symbol-table internals, or HIR enum
names.

## Producing an export

Pass a `.ving` source file to `vng --emit-ir` and redirect standard output to a
file or another process:

```bash
vng --emit-ir examples/fibonacci.ving > fibonacci.vinglish-export.json
```

Standard output is reserved for JSON so it can be piped safely. Compiler
diagnostics and failures are written to standard error.

## Versioning

Every document starts with these required top-level fields:

```json
{
  "format": "vinglish.semantic-export",
  "version": 1,
  "program": { "modules": [] }
}
```

`format` identifies this contract family. `version` is a whole-number schema
version. Consumers must reject versions they do not recognize rather than infer
their meaning. Version 1 permits additive changes only in optional fields;
incompatible changes require a new version.

## Version 1 vocabulary

Version 1 describes the semantic concepts required for first-generation analysis
and reasoning integrations:

- Programs and modules.
- Functions and parameters.
- Variables, assignments, and mutations.
- Calls and returns.
- While loops and conditionals.
- Semantic type descriptions.

Expressions preserve identifiers, literals, calls, binary and unary operations,
and collections. An `unsupported` marker preserves a source range for a source
construct not represented by this version of the contract, without leaking a
compiler-internal construct.

## Consumer requirements

Consumers should treat the contract as data, not as an executable description
of Vinglish. In particular, a consumer should:

- Check both `format` and `version` before using `program`.
- Accept missing optional fields.
- Reject unknown required schema versions.
- Preserve source spans for diagnostics and editor integrations.
- Avoid depending on field ordering or compiler implementation details.

## Ownership

The `vinglish-ir-export` crate owns the schema and its builder from compiler HIR
to the transport document. It may depend on compiler crates. External
integrations should not depend on that crate; they should implement this
published JSON contract instead.
