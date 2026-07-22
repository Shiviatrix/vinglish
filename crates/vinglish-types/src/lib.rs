pub mod env;
pub mod healer;
pub mod mir_lower;
pub mod passes;
pub mod type_pass;
pub mod validator;

pub use vinglish_hir::symbol;
pub use vinglish_hir::types::{Type, TypeVar};
pub use env::TypeEnv;
pub use mir_lower::MirLowerer;
pub use passes::{CompilerContext, CompilerPass};
pub use type_pass::{AstNodeId, TypeError, TypeInferencePass};
pub use healer::{attempt_heal, HealingRule, HealingWarning};
pub use validator::HirValidatorPass;

pub use type_pass::infer_module;
