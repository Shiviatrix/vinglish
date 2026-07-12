use eng_hir::symbol::SsaValueId;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum OwnershipState {
    #[default]
    Uninitialized,
    Owned,
    BorrowedShared(Vec<SsaValueId>), // borrowed by
    BorrowedMutable(SsaValueId),     // borrowed by
    Moved(SsaValueId),               // moved to
    Dropped,
}

impl fmt::Display for OwnershipState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OwnershipState::Uninitialized => write!(f, "Uninitialized"),
            OwnershipState::Owned => write!(f, "Owned"),
            OwnershipState::BorrowedShared(by) => {
                let ids = by
                    .iter()
                    .map(|id| format!("var_{}", id.0))
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "BorrowedShared by [{}]", ids)
            }
            OwnershipState::BorrowedMutable(by) => write!(f, "BorrowedMutable by var_{}", by.0),
            OwnershipState::Moved(to) => write!(f, "Moved to var_{}", to.0),
            OwnershipState::Dropped => write!(f, "Dropped"),
        }
    }
}
