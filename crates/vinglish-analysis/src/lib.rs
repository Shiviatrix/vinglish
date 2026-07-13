pub mod alias;
pub mod escape;
pub mod lifetime;
pub mod promotion;
pub mod validator;

pub use alias::{AliasAnalysisPass, AliasGraph};
pub use escape::{EscapeAnalysis, EscapeAnalysisPass};
pub use lifetime::{LifetimeAnalysisPass, LifetimeGraph};
pub use promotion::StackPromotionPass;
pub use validator::AnalysisValidator;
