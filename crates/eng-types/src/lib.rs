pub mod env;
pub mod passes;
pub mod type_pass;
pub mod validator;
pub mod mir_lower;

pub use type_pass::{TypeError, TypeInferencePass};
pub use passes::{CompilerContext, CompilerPass};
pub use validator::HirValidatorPass;
pub use eng_hir::types::{Type, TypeVar};
pub use eng_hir::symbol;
pub use env::TypeEnv;
pub use mir_lower::MirLowerer;

pub use type_pass::infer_module;
