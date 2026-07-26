/// TODO: Describe implementation.
pub mod backend;
/// TODO: Describe implementation.
pub mod interp;
/// TODO: Describe implementation.
pub mod lower;
/// TODO: Describe implementation.
pub mod mir_codegen;

#[cfg(test)]
mod codegen_stress;

pub use backend::Backend;
pub use interp::{InterpError, Interpreter, Value};
pub use lower::{emit_c, CEmitError};
pub use mir_codegen::{emit_mir_c, MirCEmitError};
