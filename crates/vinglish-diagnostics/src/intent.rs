use crate::diagnostic::Diagnostic;
use crate::heuristics::{polyglot, lexical};

/// The main entry point for the Heuristic Intent Engine.
/// Takes a raw diagnostic, the offending token, and the surrounding context,
/// and attempts to mutate the diagnostic into an intent-aware error.
pub fn resolve_intent(
    diag: &mut Diagnostic,
    bad_token_text: &str,
    context: &str,
) {
    // Node 1: Polyglot Interference (C, Python, Java muscle memory)
    if polyglot::check_polyglot_interference(bad_token_text, context, diag) {
        return;
    }

    // Node 2: Lexical Proximity (Typos)
    lexical::check_lexical_proximity(bad_token_text, diag);

    // (Future) Node 3: Semantic Consistency (Handled higher up in HIR/Types)
}
