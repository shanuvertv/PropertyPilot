use chrono::{DateTime, Utc};
use renewal_core::{Capability, Role};
use uuid::Uuid;

use crate::error::ServiceResult;

/// The authenticated caller. Built by the server from a bearer token on every request.
#[derive(Debug, Clone)]
pub struct Session {
    pub session_id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub email: String,
    pub role: Role,
    pub expires_at: DateTime<Utc>,
}

impl Session {
    pub fn require(&self, cap: Capability) -> ServiceResult<()> {
        Ok(self.role.require(cap)?)
    }
}
