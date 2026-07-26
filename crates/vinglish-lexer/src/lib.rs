/// TODO: Describe implementation.
pub mod lexer;
/// TODO: Describe implementation.
pub mod span;
/// TODO: Describe implementation.
pub mod token;

pub use lexer::{tokenize, LexError};
pub use span::{Span, Spanned};
pub use token::Token;
