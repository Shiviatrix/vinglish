use thiserror::Error;
use vinglish_lexer::{Span, Token};

/// TODO: Describe implementation.
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
    #[error("{message} at {span}")]
    Custom { message: String, span: Span },
}

impl ParseError {
    /// TODO: Describe implementation.
    pub fn expected(expected: impl Into<String>, found: &Token, span: Span) -> Self {
        Self::Expected {
            expected: expected.into(),
            found: found.describe().to_string(),
            span,
        }
    }

    /// TODO: Describe implementation.
    pub fn span(&self) -> Span {
        match self {
            Self::Expected { span, .. } => *span,
            Self::InvalidExpr { span } => *span,
            Self::InvalidType { span } => *span,
            Self::Custom { span, .. } => *span,
            Self::UnexpectedEof => Span::dummy(),
        }
    }
}
