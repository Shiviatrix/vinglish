use crate::diagnostic::{Diagnostic, Suggestion};
use vinglish_lexer::Span;

fn main() {
    let mut diag = Diagnostic::error("T0001", "test error", Span::dummy());
    let result = super::lexical_debug::check_lexical_proximity("funtion", &mut diag);
    println!("Result: {}", result);
    println!("Suggestions: {:?}", diag.suggestions);
}
