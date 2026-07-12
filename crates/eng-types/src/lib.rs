pub mod env;
pub mod mir_lower;
pub mod passes;
pub mod type_pass;
pub mod validator;

pub use eng_hir::symbol;
pub use eng_hir::types::{Type, TypeVar};
pub use env::TypeEnv;
pub use mir_lower::MirLowerer;
pub use passes::{CompilerContext, CompilerPass};
pub use type_pass::{TypeError, TypeInferencePass};
pub use validator::HirValidatorPass;

pub use type_pass::infer_module;
