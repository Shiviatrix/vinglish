/// TODO: Describe implementation.
pub mod env;
/// TODO: Describe implementation.
pub mod healer;
/// TODO: Describe implementation.
pub mod mir_lower;
/// TODO: Describe implementation.
pub mod passes;
/// TODO: Describe implementation.
pub mod type_pass;
/// TODO: Describe implementation.
pub mod validator;

#[cfg(test)]
mod healer_stress;

pub use env::TypeEnv;
pub use healer::{HealingRule, HealingWarning, attempt_heal};
pub use mir_lower::MirLowerer;
pub use passes::{CompilerContext, CompilerPass};
pub use type_pass::{AstNodeId, TypeError, TypeInferencePass};
pub use validator::HirValidatorPass;
pub use vinglish_hir::symbol;
pub use vinglish_hir::types::{Type, TypeVar};

pub use type_pass::infer_module;
