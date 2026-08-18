use crate::diagnostic::{Diagnostic, Suggestion};
use strsim::damerau_levenshtein;

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

/// Calculate calibrated confidence score for healing suggestions
/// This function maps raw similarity scores to more meaningful confidence percentages
/// based on empirical observations of typo patterns in programming languages
fn calibrated_confidence(raw_similarity: f64) -> f32 {
    // Apply a calibration curve to make confidence scores more meaningful
    // This prevents overconfidence in low-similarity matches while preserving
    // high confidence for clear typos

    // For very high similarity (>0.95), we're very confident
    if raw_similarity > 0.95 {
        return (95.0 + (raw_similarity - 0.95) * 100.0 * 0.5) as f32; // 95-100 range
    }

    // For high similarity (0.85-0.95), good confidence
    if raw_similarity > 0.85 {
        return (85.0 + (raw_similarity - 0.85) * 100.0 * 0.666) as f32; // 85-95 range
    }

    // For moderate similarity (0.7-0.85), lower confidence
    if raw_similarity > 0.7 {
        return (70.0 + (raw_similarity - 0.7) * 100.0) as f32; // 70-85 range
    }

    // For low similarity (<0.7), very low confidence (likely not a typo)
    (raw_similarity * 50.0) as f32 // 0-35 range
}

pub fn check_lexical_proximity(bad_token_text: &str, diag: &mut Diagnostic) -> bool {
    let mut scored: Vec<(&str, f64)> = VINGLISH_KEYWORDS
        .iter()
        .map(|s| {
            let score = similarity_score(bad_token_text, s);
            (*s, score)
        })
        .collect();

    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut found = false;
    for (candidate, score) in scored.iter().take(3) {
        // Use calibrated confidence with safety threshold
        let confidence = calibrated_confidence(*score);

        // Safety mechanism: Only suggest if confidence is above minimum threshold
        // This prevents suggesting corrections for completely unrelated words
        if confidence > 60.0 {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_similarity_scoring() {
        // Test transposition
        let score = similarity_score("funtion", "function");
        println!("funtion -> function: {}", score);
        assert!(score > 0.8);

        // Test missing character
        let score = similarity_score("fction", "function");
        println!("fction -> function: {}", score);
        assert!(score > 0.7);

        // Test duplicated character
        let score = similarity_score("funcction", "function");
        println!("funcction -> function: {}", score);
        assert!(score > 0.7);

        // Test case error
        let score = similarity_score("FUNCTION", "function");
        println!("FUNCTION -> function: {}", score);
        assert!(score > 0.9);

        // Test completely different word
        let score = similarity_score("hello", "function");
        println!("hello -> function: {}", score);
        assert!(score < 0.5);
    }

    #[test]
    fn test_calibrated_confidence() {
        // Test that confidence scores are reasonable
        let conf = calibrated_confidence(0.96); // Very high similarity
        assert!((95.0..=100.0).contains(&conf));

        let conf = calibrated_confidence(0.90); // High similarity
        assert!((85.0..95.0).contains(&conf));

        let conf = calibrated_confidence(0.75); // Moderate similarity
        assert!((70.0..85.0).contains(&conf));

        let conf = calibrated_confidence(0.5); // Low similarity
        assert!(conf < 35.0);
    }

    #[test]
    fn test_lexical_proximity() {
        // Should suggest function for funtion (transposition) - high confidence
        let mut diag1 = crate::diagnostic::Diagnostic::error(
            "T0001",
            "test error",
            vinglish_lexer::Span::dummy(),
        );
        let result = check_lexical_proximity("funtion", &mut diag1);
        assert!(result);
        assert!(!diag1.suggestions.is_empty());
        // Should have high confidence (>85)
        assert!(diag1.suggestions[0].confidence.unwrap() > 85.0);
        assert_eq!(
            diag1.suggestions[0].replacement.as_ref().unwrap(),
            "function"
        );

        // Should suggest function for fction (missing char) - moderate confidence
        let mut diag2 = crate::diagnostic::Diagnostic::error(
            "T0001",
            "test error",
            vinglish_lexer::Span::dummy(),
        );
        let result = check_lexical_proximity("fction", &mut diag2);
        assert!(result);
        assert!(!diag2.suggestions.is_empty());
        #[allow(dead_code)]
        let conf = diag2.suggestions[0].confidence.unwrap();
        assert!(conf > 70.0);
        assert_eq!(
            diag2.suggestions[0].replacement.as_ref().unwrap(),
            "function"
        );

        // Should NOT suggest function for hello (unrelated word) - low confidence
        let mut diag3 = crate::diagnostic::Diagnostic::error(
            "T0001",
            "test error",
            vinglish_lexer::Span::dummy(),
        );
        let result = check_lexical_proximity("hello", &mut diag3);
        assert!(!result); // Should not find any suggestions
        assert!(diag3.suggestions.is_empty());
    }
}
