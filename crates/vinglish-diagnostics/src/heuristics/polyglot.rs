use crate::diagnostic::{Diagnostic, Suggestion};

/// Checks if the provided code text looks like muscle memory from another language
/// and mutates the diagnostic to provide an intent-aware English suggestion.
pub fn check_polyglot_interference(bad_token_text: &str, context: &str, diag: &mut Diagnostic) -> bool {
    // Check for C/Python style variable assignment (using '=' instead of 'be')
    if bad_token_text == "=" && (context.contains("let ") || context.contains("mutable ")) {
        diag.suggestions.push(
            Suggestion::new("Vinglish uses 'be' for assignments to read like English prose.")
                .with_replacement("be")
                .with_confidence(95.0)
        );
        diag.message = "Unexpected C-style assignment operator '='.".to_string();
        return true;
    }

    // Check for Python style function declaration ('def' instead of 'function')
    if bad_token_text == "def" {
        diag.suggestions.push(
            Suggestion::new("Vinglish uses 'function' to declare functions.")
                .with_replacement("function")
                .with_confidence(98.0)
        );
        diag.message = "Unexpected Python-style 'def' keyword.".to_string();
        return true;
    }

    // Check for C/Java style integer types ('int' instead of 'number')
    if bad_token_text == "int" || bad_token_text == "float" || bad_token_text == "double" {
        diag.suggestions.push(
            Suggestion::new("Vinglish uses 'number' for numeric types.")
                .with_replacement("number")
                .with_confidence(90.0)
        );
        diag.message = format!("Unexpected C-style numeric type '{}'.", bad_token_text);
        return true;
    }
    
    // Check for braces instead of begin/end
    if bad_token_text == "{" {
        diag.suggestions.push(
            Suggestion::new("Vinglish uses 'begin' and 'end' for block scopes.")
                .with_replacement("begin")
                .with_confidence(85.0)
        );
        diag.message = "Unexpected C-style opening brace '{'.".to_string();
        return true;
    }

    // Check for Rust/C++ trailing return types ('->' instead of 'returns')
    if bad_token_text == "->" && context.contains("function") {
        diag.suggestions.push(
            Suggestion::new("Vinglish uses 'returns' to declare a function's return type.")
                .with_replacement("returns")
                .with_confidence(98.0)
        );
        diag.message = "Unexpected Rust/C++ style trailing return '->'.".to_string();
        return true;
    }

    // Check for C/Python style while loop ('while' without 'repeat')
    if bad_token_text == "while" && !context.contains("repeat") {
        diag.suggestions.push(
            Suggestion::new("Vinglish loops read like prose: use 'repeat while'.")
                .with_replacement("repeat while")
                .with_confidence(98.0)
        );
        diag.message = "Unexpected C/Python style 'while' loop.".to_string();
        return true;
    }

    // Check for assignment '=' inside 'if' conditions (should be 'is' or '==')
    if bad_token_text == "=" && context.contains("if ") {
        diag.suggestions.push(
            Suggestion::new("Vinglish uses 'is' or '==' for equality checks, not '='.")
                .with_replacement("is")
                .with_confidence(95.0)
        );
        diag.message = "Unexpected assignment '=' in condition.".to_string();
        return true;
    }

    false
}
