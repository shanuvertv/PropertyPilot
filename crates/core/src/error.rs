use crate::roles::{Capability, Role};

/// A role attempted something its permission matrix does not allow (PLAN.md §6.7).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{role} is not allowed to {capability}")]
pub struct PermissionDenied {
    pub role: Role,
    pub capability: Capability,
}

/// A status change that the state machine does not permit (PLAN.md §6.1–6.3).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("cannot move {entity} from {from} to {to}")]
pub struct TransitionError {
    pub entity: &'static str,
    pub from: String,
    pub to: String,
}
