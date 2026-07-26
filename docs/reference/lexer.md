# Lexer

The Vinglish lexer tokenizes source text into a flat sequence of `Spanned<Token>` values.

---

## Purpose

Convert a `.ving` source string into tokens for the parser. Handle indentation-based block structure.

---

## Responsibilities

- Measure leading whitespace per line and emit `Indent`/`Dedent` tokens.
- Recognize keywords, natural-language aliases, identifiers, and literals.
- Recognize operators and punctuation.
- Strip comments (`--` and `#` line comments).
- Report lexer errors without halting (error recovery).

---

## Design

### Entry Point

`tokenize(src: &str) -> (Vec<Spanned<Token>>, Vec<LexError>)`

Implemented in `vinglish-lexer/src/lexer.rs`.

### Algorithm

1. Iterate over each line of `src`.
2. For each non-blank, non-comment line:
   - Measure indentation (spaces + tabs, where tab = 4 spaces).
   - Compare with the top of `indent_stack`:
     - Greater: push new level, emit `Indent`.
     - Less: pop levels, emit `Dedent` for each.
   - Call `lex_line()` on the trimmed content.
3. Emit `Newline` after each physical line.
4. At end of input: emit `Dedent` for all remaining open blocks, then `EOF`.

### `lex_line()`

Scans a single line for:
- **Comments**: `#` or `--` halts scanning for the rest of the line.
- **String literals**: Delimited by `"`. Supports escape sequences: `\n`, `\t`, `\r`, `\0`, `\"`, `\'`, `\\`.
- **Numeric literals**: Integer (`i64`) and float (`f64`). Underscore `_` is allowed as a digit separator. A `.` followed by a digit triggers float parsing.
- **Identifiers and keywords**: `Token::from_word()` maps keywords and aliases. Unrecognized words become `Token::Ident(word)`.
- **Operators**: Single and multi-character (`+=`, `-=`, `*=`, `/=`, `->`, `=>`, `==`, `!=`, `<=`, `>=`). Bare `=` is treated as `Token::Be`.

### Natural-Language Aliases

| Input | Token |
|---|---|
| `compute`, `calculate`, `determine` | `Ident("calculate")` |
| `modify`, `mutate` | `Ident("mutate")` |
| `create`, `make` | `Ident("create")` |
| `destroy`, `delete` | `Ident("delete")` |
| `show`, `display`, `print`, `println` | `Ident(original word)` |

---

## Data Flow

**Input**: `&str` (source text).

**Output**: `(Vec<Spanned<Token>>, Vec<LexError>)`.

---

## Public API

| Item | Description |
|---|---|
| `tokenize(src)` | Main entry point |
| `Token` | Enum: 70+ variants for keywords, literals, operators, punctuation, structural tokens |
| `Token::from_word(s)` | Maps a word to a keyword token or `None` |
| `Token::describe(&self)` | Human-readable description for error messages |
| `LexError` | `UnexpectedChar`, `UnterminatedString`, `InvalidNumber` |
| `Span` | Half-open byte range `[start, end)` |
| `Spanned<T>` | Value `T` paired with a `Span` |

---

## Examples

From `vinglish-lexer/src/lexer.rs` tests:

```
Input: "let age be 25"
Tokens: [Let, Ident("age"), Be, Integer(25)]
```

```
Input: "if x\n    return y\n"
Tokens: [If, Ident("x"), Newline, Indent, Return, Ident("y"), Newline, Dedent, ...]
```

---

## Limitations

- Tab width is hardcoded at 4 spaces.
- No multi-line string literals.
- No raw string literals.
- `//` is treated as a C-style comment terminator (breaks the rest of the line).

---

## Related Components

- [Reference: Parser](parser.md)
- [Compiler Pipeline](../explanation/compiler-pipeline.md)
