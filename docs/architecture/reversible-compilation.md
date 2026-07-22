# Reversible compilation and deterministic recovery

## Non-negotiable boundary

The current C backend accepts `vinglish_parser::ast::Module`; it has no MIR
instruction identity to annotate. It therefore cannot truthfully emit a
MIR-to-C mapping yet. The reversible backend must be moved behind a MIR input
API, after SSA construction and before C text emission. Annotating the current
AST backend would only create an AST-to-C map and is not a solution to C -> MIR.

An instruction ID alone is also insufficient: IDs name nodes but do not encode
operands, type IDs, phi predecessor edges, block terminators, locals, or symbol
table identities. Perfect reconstruction requires a complete canonical MIR
snapshot, with the per-statement ID used as its C location index.

## C carrier schema

Place one ordinary C comment immediately before every C statement produced from
an instruction:

```c
/* vinglish:mir v=1 module=SHA256 fn=12 bb=3 inst=8 op=BinaryAdd payload=HEX */
tmp_8 = tmp_4 + tmp_7;
```

`payload` is canonical hex encoding of the typed instruction record. A file
header carries canonical module data (function signatures, local/value types,
symbol dictionary, CFG layout, source spelling/provenance and a hash). Each tag
repeats its identity and record. Comments are discarded in preprocessing and
have zero runtime, ABI, object-code, and optimizer cost in GCC and Clang.

The generated-C parser must reject missing, duplicate, version-mismatched, or
fingerprint-mismatched records. It must never infer MIR from C syntax. That
keeps arbitrary edits to generated C explicitly unsupported rather than silently
producing a different program.

`vinglish-decompile` supplies the deterministic carrier parser and an explicit
`MirSnapshotDecoder` boundary. The codegen crate must own the paired encoder;
both must use a fixed field order, explicit integer widths, escaping rules, and
a version number.

## Reversible lowering contracts

The lowering interfaces must retain lossless provenance instead of relying on
spans or pretty-printed names:

```rust
trait ReversibleLower<From, To> {
    type Witness: Clone + Eq + std::fmt::Debug;
    type Error: std::error::Error + Send + Sync + 'static;
    fn lower(&self, from: &From) -> Result<(To, Self::Witness), Self::Error>;
    fn raise(&self, to: &To, witness: &Self::Witness) -> Result<From, Self::Error>;
}
```

`Witness` must preserve source-level distinctions erased by HIR/MIR: natural
language operator spelling, explicit versus inferred type annotations, names,
field order, macro expansion origin, desugarings, and layout-independent source
order. `raise(lower(x).0, lower(x).1)` must equal `x` under a documented AST
normalization. Without this witness MIR -> HIR -> AST can be semantically
equivalent but cannot be character-for-character or syntax-choice perfect.

The reverse pipeline is `C comments -> canonical MIR -> HIR plus witness -> AST
plus witness -> formatter`. The formatter, not C parsing, writes `.ving`.

## Diagnostic healing boundary

`TypeError` currently holds only message and span. Before healing is enabled,
add a structured `TypeConstraint { expected, actual, span, ast_node_id }` and
make the type pass report it at unification sites. Resolve the node ID to a
mutable AST slot, run only bounded `Healer` rules, then re-run the normal type
checker on the rebuilt module. Commit exactly one candidate only on success and
emit a warning containing the selected rule. Otherwise restore the original AST
and retain the original fatal diagnostic.

The included `vinglish_types::healer` is the rule engine and safe mutation hook.
It deliberately does not parse diagnostic text, mutate through aliases, or
auto-heal ownership errors; borrow repairs require a separate proof-preserving
rule set in `vinglish-own`.
