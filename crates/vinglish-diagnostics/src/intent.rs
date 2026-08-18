use crate::diagnostic::Diagnostic;
use crate::heuristics::lexical;

/// The main entry point for the Heuristic Intent Engine.
/// Takes a raw diagnostic, the offending token, and the surrounding context,
/// and attempts to mutate the diagnostic into an intent-aware error.
pub fn resolve_intent(diag: &mut Diagnostic, bad_token_text: &str, _context: &str) {
    // Lexical proximity (typos)
    lexical::check_lexical_proximity(bad_token_text, diag);
}
