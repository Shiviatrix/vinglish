use crate::diagnostic::{Diagnostic, Suggestion};
use strsim::{damerau_levenshtein, jaro_winkler};

const VINGLISH_KEYWORDS: &[&str] = &[
    "function", "let", "be", "mutable", "return", "if", "else", "begin", "end", "number", "string",
    "boolean", "true", "false", "and", "or", "not", "is", "below", "above",
];

/// Calculate similarity score using multiple algorithms for better typo detection
fn similarity_score(s1: &str, s2: &str) -> f64 {
    // Use Damerau-Levenshtein which handles transpositions well
    let distance = damerau_levenshtein(s1, s2);
    let max_len = s1.len().max(s2.len()) as f64;
    
    if max_len == 0.0 {
        return 1.0;
    }
    
    // Convert distance to similarity (0-1 range, higher is better)
    let similarity = 1.0 - (distance as f64 / max_len);
    
    // Boost score for exact case-insensitive matches
    if s1.eq_ignore_ascii_case(s2) && s1.len() == s2.len() {
        return similarity.max(0.95);
    }
    
    similarity
}

pub fn check_lexical_proximity(bad_token_text: &str, diag: &mut Diagnostic) -> bool {
    let mut scored: Vec<(&str, f64)> = VINGLISH_KEYWORDS
        .iter()
        .map(|s| {
            let score = similarity_score(bad_token_text, *s);
            (*s, score)
        })
        .collect();

    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    println!("Scoring for '{}':", bad_token_text);
    for (candidate, score) in &scored {
        println!("  {}: {}", candidate, score);
    }

    let mut found = false;
    for (candidate, score) in scored.iter().take(3) {
        // High confidence threshold for typos
        if *score > 0.8 {
            let confidence = (*score * 100.0) as f32;
            diag.suggestions.push(
                Suggestion::new(format!("Did you mean '{}'?", candidate))
                    .with_replacement(candidate.to_string())
                    .with_confidence(confidence),
            );
            found = true;
        }
    }

    if found {
        diag.message = format!(
            "Unknown token '{}' closely matches a keyword.",
            bad_token_text
        );
    }

    found
}
