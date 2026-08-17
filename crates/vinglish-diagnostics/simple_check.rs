use crate::diagnostic::{Diagnostic, Suggestion};
use crate::heuristics::lexical::check_lexical_proximity;
use vinglish_lexer::Span;

fn main() {
    let mut diag = Diagnostic::error("T0001", "test error", Span::dummy());
    let result = check_lexical_proximity("funtion", &mut diag);
    println!("Result: {}", result);
    println!("Suggestions len: {}", diag.suggestions.len());
    if !diag.suggestions.is_empty() {
        println!("First suggestion: {:?}", diag.suggestions[0]);
    }
}
