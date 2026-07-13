use crate::diagnostic::{Diagnostic, Suggestion};
use strsim::jaro_winkler;

const ENGLIST_KEYWORDS: &[&str] = &[
    "function", "let", "be", "mutable", "return", "if", "else", "begin", "end",
    "number", "string", "boolean", "true", "false", "and", "or", "not", "is", "below", "above"
];

pub fn check_lexical_proximity(bad_token_text: &str, diag: &mut Diagnostic) -> bool {
    let mut scored: Vec<(&str, f64)> = ENGLIST_KEYWORDS
        .iter()
        .map(|s| (*s, jaro_winkler(bad_token_text, s)))
        .collect();
    
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut found = false;
    for (candidate, score) in scored.iter().take(3) {
        // High confidence threshold for typos
        if *score > 0.85 {
            let confidence = (score * 100.0) as f32;
            diag.suggestions.push(
                Suggestion::new(format!("Did you mean '{}'?", candidate))
                    .with_replacement(candidate.to_string())
                    .with_confidence(confidence),
            );
            found = true;
        }
    }
    
    if found {
        diag.message = format!("Unknown token '{}' closely matches a keyword.", bad_token_text);
    }
    
    found
}
