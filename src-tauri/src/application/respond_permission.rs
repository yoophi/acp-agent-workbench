use anyhow::Result;

use crate::ports::permission::{PermissionDecision, PermissionDecisionPort};

pub struct RespondPermissionUseCase<P>
where
    P: PermissionDecisionPort,
{
    permissions: P,
}

impl<P> RespondPermissionUseCase<P>
where
    P: PermissionDecisionPort,
{
    pub fn new(permissions: P) -> Self {
        Self { permissions }
    }

    pub async fn execute(&self, permission_id: &str, option_id: String) -> Result<()> {
        self.permissions
            .respond(permission_id, PermissionDecision { option_id })
            .await
    }
}
