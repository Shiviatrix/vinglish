/// TODO: Describe implementation.
pub mod alias;
/// TODO: Describe implementation.
pub mod escape;
/// TODO: Describe implementation.
pub mod lifetime;
/// TODO: Describe implementation.
pub mod promotion;
/// TODO: Describe implementation.
pub mod validator;

pub use alias::{AliasAnalysisPass, AliasGraph};
pub use escape::{EscapeAnalysis, EscapeAnalysisPass};
pub use lifetime::{LifetimeAnalysisPass, LifetimeGraph};
pub use promotion::StackPromotionPass;
pub use validator::AnalysisValidator;
