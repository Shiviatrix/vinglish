use crate::state::OwnershipState;
use vinglish_hir::symbol::SsaValueId;
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Default)]
pub struct OwnershipGraph {
    states: HashMap<SsaValueId, OwnershipState>,
    pub history: HashMap<SsaValueId, Vec<OwnershipState>>,
}

impl OwnershipGraph {
    pub fn new() -> Self {
        Self {
            states: HashMap::new(),
            history: HashMap::new(),
        }
    }

    pub fn set_state(&mut self, var: SsaValueId, state: OwnershipState) {
        self.states.insert(var, state.clone());
        self.history.entry(var).or_default().push(state);
    }

    pub fn get_state(&self, var: SsaValueId) -> OwnershipState {
        self.states
            .get(&var)
            .cloned()
            .unwrap_or(OwnershipState::Uninitialized)
    }

    pub fn is_owned(&self, var: SsaValueId) -> bool {
        matches!(self.get_state(var), OwnershipState::Owned)
    }
}

impl fmt::Display for OwnershipGraph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut vars: Vec<_> = self.history.keys().copied().collect();
        vars.sort_by_key(|v| v.0);

        for var in vars {
            writeln!(f, "var_{}", var.0)?;
            let hist = &self.history[&var];
            for (i, state) in hist.iter().enumerate() {
                if i > 0 {
                    writeln!(f, "↓")?;
                }
                writeln!(f, "{}", state)?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}
