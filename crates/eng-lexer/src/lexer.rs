use crate::span::{Span, Spanned};
use crate::token::Token;
use thiserror::Error;

/// Lexer errors — all recoverable; we collect them and continue.
#[derive(Debug, Clone, Error)]
pub enum LexError {
    #[error("unexpected character '{ch}' at offset {offset}")]
    UnexpectedChar { ch: char, offset: u32 },
    #[error("unterminated string literal starting at offset {offset}")]
    UnterminatedString { offset: u32 },
    #[error("invalid numeric literal '{text}' at offset {offset}")]
    InvalidNumber { text: String, offset: u32 },
}

/// Tokenise an Englist source string.
///
/// The lexer uses a *line-by-line* indentation model:
/// - Each non-blank, non-comment line is measured for leading spaces/tabs.
/// - Indent level changes emit synthetic `Indent` / `Dedent` tokens.
/// - Tab = 4 spaces.
///
/// Both `begin/end` block syntax and indentation-based blocks are supported.
/// The lexer itself is indent-aware; the parser decides which style to use.
pub fn tokenize(src: &str) -> (Vec<Spanned<Token>>, Vec<LexError>) {
    let mut tokens: Vec<Spanned<Token>> = Vec::new();
    let mut errors: Vec<LexError> = Vec::new();
    let mut indent_stack: Vec<usize> = vec![0];
    let mut byte_offset: u32 = 0;

    for raw_line in src.lines() {
        let line_len = raw_line.len() as u32;

        // ── Measure indentation ───────────────────────────────────────────────
        let indent: usize = raw_line
            .chars()
            .take_while(|c| *c == ' ' || *c == '\t')
            .map(|c| if c == '\t' { 4 } else { 1 })
            .sum();

        let trimmed = raw_line.trim_start();

        // Skip blank lines and pure comment lines (don't affect indent).
        let is_blank_or_comment =
            trimmed.is_empty() || trimmed.starts_with("--") || trimmed.starts_with('#');

        if !is_blank_or_comment {
            let current = *indent_stack.last().unwrap_or(&0);

            if indent > current {
                // Opening a new indented block
                indent_stack.push(indent);
                tokens.push(Spanned::new(
                    Token::Indent,
                    Span::new(byte_offset, byte_offset + indent as u32),
                ));
            } else {
                // Possibly closing one or more blocks
                while *indent_stack.last().unwrap_or(&0) > indent {
                    indent_stack.pop();
                    tokens.push(Spanned::new(
                        Token::Dedent,
                        Span::new(byte_offset, byte_offset),
                    ));
                }
            }

            // ── Lex the content of this line ──────────────────────────────────
            let content_start = byte_offset + indent as u32;
            let (mut line_toks, mut line_errs) = lex_line(trimmed, content_start);
            tokens.append(&mut line_toks);
            errors.append(&mut line_errs);
        }

        // Newline at end of physical line
        byte_offset += line_len + 1; // +1 for '\n'
        tokens.push(Spanned::new(
            Token::Newline,
            Span::new(byte_offset - 1, byte_offset),
        ));
    }

    // Close any remaining open blocks
    while indent_stack.len() > 1 {
        indent_stack.pop();
        tokens.push(Spanned::new(
            Token::Dedent,
            Span::new(byte_offset, byte_offset),
        ));
    }

    tokens.push(Spanned::new(
        Token::EOF,
        Span::new(byte_offset, byte_offset),
    ));

    (tokens, errors)
}

/// Lex a single line (no indentation logic, just tokens).
/// `base_offset` is the byte position of the first character.
fn lex_line(line: &str, base_offset: u32) -> (Vec<Spanned<Token>>, Vec<LexError>) {
    let chars: Vec<char> = line.chars().collect();
    let mut pos = 0usize;
    let mut tokens = Vec::new();
    let mut errors = Vec::new();

    macro_rules! offs {
        () => {
            base_offset + pos as u32
        };
    }

    macro_rules! span_from {
        ($start:expr) => {
            Span::new($start, offs!())
        };
    }

    while pos < chars.len() {
        // Skip horizontal whitespace
        while pos < chars.len() && (chars[pos] == ' ' || chars[pos] == '\t') {
            pos += 1;
        }
        if pos >= chars.len() {
            break;
        }

        let start_offs = offs!();
        let ch = chars[pos];

        // ── Comments ─────────────────────────────────────────────────────────
        if ch == '#' || (ch == '-' && chars.get(pos + 1) == Some(&'-')) {
            break; // Rest of line is a comment
        }

        // ── String literals ───────────────────────────────────────────────────
        if ch == '"' {
            pos += 1;
            let mut s = String::new();
            let mut closed = false;
            while pos < chars.len() {
                match chars[pos] {
                    '"' => {
                        pos += 1;
                        closed = true;
                        break;
                    }
                    '\\' => {
                        pos += 1;
                        if pos < chars.len() {
                            s.push(escape_char(chars[pos]));
                            pos += 1;
                        }
                    }
                    c => {
                        s.push(c);
                        pos += 1;
                    }
                }
            }
            if !closed {
                errors.push(LexError::UnterminatedString { offset: start_offs });
            } else {
                tokens.push(Spanned::new(Token::StringLit(s), span_from!(start_offs)));
            }
            continue;
        }

        // ── Numeric literals ──────────────────────────────────────────────────
        if ch.is_ascii_digit() {
            let mut num = String::new();
            let mut is_float = false;
            while pos < chars.len() {
                let c = chars[pos];
                if c.is_ascii_digit() {
                    num.push(c);
                    pos += 1;
                } else if c == '_' {
                    pos += 1; // separator, ignore
                } else if c == '.'
                    && !is_float
                    && chars
                        .get(pos + 1)
                        .map(|p| p.is_ascii_digit())
                        .unwrap_or(false)
                {
                    is_float = true;
                    num.push('.');
                    pos += 1;
                } else {
                    break;
                }
            }
            let span = span_from!(start_offs);
            if is_float {
                match num.parse::<f64>() {
                    Ok(f) => tokens.push(Spanned::new(Token::Float(f), span)),
                    Err(_) => errors.push(LexError::InvalidNumber {
                        text: num,
                        offset: start_offs,
                    }),
                }
            } else {
                match num.parse::<i64>() {
                    Ok(i) => tokens.push(Spanned::new(Token::Integer(i), span)),
                    Err(_) => errors.push(LexError::InvalidNumber {
                        text: num,
                        offset: start_offs,
                    }),
                }
            }
            continue;
        }

        // ── Identifiers and keywords ──────────────────────────────────────────
        if ch.is_alphabetic() || ch == '_' {
            let mut word = String::new();
            while pos < chars.len() && (chars[pos].is_alphanumeric() || chars[pos] == '_') {
                word.push(chars[pos]);
                pos += 1;
            }
            let span = span_from!(start_offs);
            let tok = Token::from_word(&word).unwrap_or(Token::Ident(word));
            tokens.push(Spanned::new(tok, span));
            continue;
        }

        // ── Operators and punctuation ─────────────────────────────────────────
        pos += 1; // consume `ch`
        let next = chars.get(pos).copied();
        let tok = match ch {
            '+' => {
                if next == Some('=') {
                    pos += 1;
                    Token::PlusEq
                } else {
                    Token::Plus
                }
            }
            '-' => {
                if next == Some('>') {
                    pos += 1;
                    Token::Arrow
                } else if next == Some('=') {
                    pos += 1;
                    Token::MinusEq
                } else {
                    Token::Minus
                }
            }
            '*' => {
                if next == Some('=') {
                    pos += 1;
                    Token::StarEq
                } else {
                    Token::Star
                }
            }
            '/' => {
                if next == Some('=') {
                    pos += 1;
                    Token::SlashEq
                } else if next == Some('/') {
                    break;
                }
                // C-style comment
                else {
                    Token::Slash
                }
            }
            '%' => Token::Percent,
            '=' => {
                if next == Some('=') {
                    pos += 1;
                    Token::Eq
                } else if next == Some('>') {
                    pos += 1;
                    Token::FatArrow
                } else {
                    Token::Be
                } // bare `=` treated as `be`
            }
            '!' => {
                if next == Some('=') {
                    pos += 1;
                    Token::NotEq
                } else {
                    Token::Bang
                }
            }
            '|' => Token::Pipe,
            '?' => Token::QuestionMark,
            '<' => {
                if next == Some('=') {
                    pos += 1;
                    Token::LtEq
                } else {
                    Token::Lt
                }
            }
            '>' => {
                if next == Some('=') {
                    pos += 1;
                    Token::GtEq
                } else {
                    Token::Gt
                }
            }
            '.' => Token::Dot,
            ',' => Token::Comma,
            ':' => Token::Colon,
            ';' => Token::Semicolon,
            '(' => Token::LParen,
            ')' => Token::RParen,
            '[' => Token::LBracket,
            ']' => Token::RBracket,
            '{' => Token::LBrace,
            '}' => Token::RBrace,
            other => {
                errors.push(LexError::UnexpectedChar {
                    ch: other,
                    offset: start_offs,
                });
                continue;
            }
        };
        tokens.push(Spanned::new(tok, span_from!(start_offs)));
    }

    (tokens, errors)
}

fn escape_char(c: char) -> char {
    match c {
        'n' => '\n',
        't' => '\t',
        'r' => '\r',
        '0' => '\0',
        '"' => '"',
        '\'' => '\'',
        '\\' => '\\',
        other => other,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(src: &str) -> Vec<Token> {
        let (ts, errs) = tokenize(src);
        assert!(errs.is_empty(), "lex errors: {:?}", errs);
        ts.into_iter()
            .filter(|t| !matches!(t.node, Token::Newline | Token::EOF))
            .map(|t| t.node)
            .collect()
    }

    #[test]
    fn simple_let() {
        let result = toks("let age be 25");
        assert_eq!(
            result,
            vec![
                Token::Let,
                Token::Ident("age".into()),
                Token::Be,
                Token::Integer(25)
            ]
        );
    }

    #[test]
    fn string_literal() {
        let result = toks(r#"print("Hello, World!")"#);
        assert!(matches!(&result[0], Token::Ident(s) if s == "print"));
        assert_eq!(result[1], Token::LParen);
        assert_eq!(result[2], Token::StringLit("Hello, World!".into()));
        assert_eq!(result[3], Token::RParen);
    }

    #[test]
    fn indentation() {
        let src = "if x\n    return y\n";
        let (ts, _) = tokenize(src);
        let kinds: Vec<_> = ts.iter().map(|t| &t.node).collect();
        assert!(kinds.contains(&&Token::Indent));
        assert!(kinds.contains(&&Token::Dedent));
    }

    #[test]
    fn operators() {
        let result = toks("x += 1");
        assert_eq!(
            result,
            vec![Token::Ident("x".into()), Token::PlusEq, Token::Integer(1)]
        );
    }

    #[test]
    fn float_literal() {
        let result = toks("3.14");
        assert_eq!(result, vec![Token::Float(3.14)]);
    }

    #[test]
    fn natural_language_alias() {
        // `compute` and `calculate` both lex to the same Ident
        let r1 = toks("compute tax");
        let r2 = toks("calculate tax");
        assert_eq!(r1[0], r2[0]);
    }

    #[test]
    fn comments_stripped() {
        let result = toks("let x be 1 -- this is a comment");
        assert_eq!(
            result,
            vec![
                Token::Let,
                Token::Ident("x".into()),
                Token::Be,
                Token::Integer(1)
            ]
        );
    }
}
