use eng_lexer::{Span, Token};
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum ParseError {
    #[error("expected {expected}, found {found} at {span}")]
    Expected {
        expected: String,
        found: String,
        span: Span,
    },
    #[error("unexpected end of file")]
    UnexpectedEof,
    #[error("invalid expression at {span}")]
    InvalidExpr { span: Span },
    #[error("invalid type expression at {span}")]
    InvalidType { span: Span },
}

impl ParseError {
    pub fn expected(expected: impl Into<String>, found: &Token, span: Span) -> Self {
        Self::Expected {
            expected: expected.into(),
            found: found.describe().to_string(),
            span,
        }
    }

    pub fn span(&self) -> Span {
        match self {
            Self::Expected { span, .. }
            | Self::InvalidExpr { span }
            | Self::InvalidType { span } => *span,
            Self::UnexpectedEof => Span::dummy(),
        }
    }
}
