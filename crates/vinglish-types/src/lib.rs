pub mod env;
pub mod healer;
pub mod mir_lower;
pub mod passes;
pub mod type_pass;
pub mod validator;

pub use env::TypeEnv;
pub use healer::{HealingRule, HealingWarning, attempt_heal};
pub use mir_lower::MirLowerer;
pub use passes::{CompilerContext, CompilerPass};
pub use type_pass::{AstNodeId, TypeError, TypeInferencePass};
pub use validator::HirValidatorPass;
pub use vinglish_hir::symbol;
pub use vinglish_hir::types::{Type, TypeVar};

pub use type_pass::infer_module;
pub mod foreign;
