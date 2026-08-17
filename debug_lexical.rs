extern crate vinglish_diagnostics;
use vinglish_diagnostics::diagnostic::{Diagnostic, Severity};
use vinglish_diagnostics::heuristics::lexical::check_lexical_proximity;
use vinglish_lexer::Span;

fn main() {
    println!("Testing lexical proximity...");
    
    let mut diag = Diagnostic::new(
        Severity::Error,
        "T0001".to_string(),
        "test error".to_string(),
        Span::dummy(),
    );
    
    let result = check_lexical_proximity("funtion", &mut diag);
    println!("Result: {}", result);
    println!("Suggestions count: {}", diag.suggestions.len());
    if !diag.suggestions.is_empty() {
        println!("First suggestion: {}", diag.suggestions[0].label);
        println!("Replacement: {:?}", diag.suggestions[0].replacement);
        println!("Confidence: {:?}", diag.suggestions[0].confidence);
    }
    
    // Test what the original test expects
    assert!(result, "check_lexical_proximity should return true for 'funtion'");
    assert!(!diag.suggestions.is_empty(), "should have suggestions");
    assert_eq!(diag.suggestions[0].replacement.as_ref().unwrap(), "function");
    println!("All assertions passed!");
}
