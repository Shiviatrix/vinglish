# Diagnostics

The diagnostics crate provides structured error and warning reporting.

---

## Purpose

Format compiler errors and warnings with source context, suggestions, and intent resolution.

---

## Responsibilities

- Represent diagnostics with severity, error code, message, span, and optional context.
- Render diagnostics to terminal output with source line excerpts.
- Resolve user intent from failed parse tokens.
- Detect polyglot errors (code from other languages).

---

## Design

Defined in `vinglish-diagnostics/src/`.

### Diagnostic Type

```rust
pub struct Diagnostic {
    pub severity: Severity,    // Error, Warning, Info
    pub code: String,          // e.g., "P0001", "T0001"
    pub message: String,
    pub span: Span,
    pub source_line: Option<String>,
    pub suggestions: Vec<Suggestion>,
    // ... additional fields
}
```

### Construction

- `Diagnostic::error(code, message, span)` — creates an error diagnostic.
- `Diagnostic::warning(code, message, span)` — creates a warning diagnostic.
- `diag.with_note(text)` — adds a note.
- `diag.enrich(&src)` — populates `source_line` from source text.

### Rendering

`render(diagnostics: &[Diagnostic], filename: &str) -> String` formats diagnostics for stderr output, including the file name, line/column, error message, and source context.

### Intent Resolution

`intent::resolve_intent(&mut diag, &found, &line)` analyzes the failed token and source line to suggest corrections for parse errors.

### Heuristics

- `heuristics::lexical` — lexical error heuristics.
- `heuristics::polyglot` — detects code patterns from other languages (e.g., Python, JavaScript, C) and suggests Vinglish equivalents.

---

## Error Codes

| Code | Source | Description |
|---|---|---|
| `P0001` | Parser | Parse error |
| `T0001` | Type inference | Type error |
| `T1001` | Type healing | Automatic type repair warning |
| `O0001` | Ownership | Ownership violation |

---

## Related Components

- [Reference: Lexer](lexer.md)
- [Reference: Parser](parser.md)
- [Compiler Pipeline](../explanation/compiler-pipeline.md)
