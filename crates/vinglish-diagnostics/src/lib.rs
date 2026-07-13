pub mod diagnostic;
pub mod renderer;

pub use diagnostic::{Diagnostic, Severity, Suggestion};
pub use renderer::render;
pub mod intent;
pub mod heuristics;
