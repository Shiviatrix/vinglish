use eng_parser::ast::Module;
use std::path::Path;

/// Trait for Englist compilation backends.
pub trait Backend {
    /// Compile the module and write a binary to `output`.
    fn compile(
        &self,
        module: &Module,
        src: &str,
        output: &Path,
    ) -> Result<(), Box<dyn std::error::Error>>;
    fn name(&self) -> &'static str;
}
