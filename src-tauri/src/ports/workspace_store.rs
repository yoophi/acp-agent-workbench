use anyhow::Result;
use std::future::Future;

use crate::domain::workspace::{CheckoutId, Workspace, WorkspaceCheckout, WorkspaceId};

pub trait WorkspaceStore: Clone + Send + Sync + 'static {
    fn list_workspaces(&self) -> impl Future<Output = Result<Vec<Workspace>>> + Send;

    fn get_workspace(&self, id: &str) -> impl Future<Output = Result<Option<Workspace>>> + Send;

    fn list_checkouts(
        &self,
        workspace_id: &str,
    ) -> impl Future<Output = Result<Vec<WorkspaceCheckout>>> + Send;

    fn get_checkout(
        &self,
        id: &str,
    ) -> impl Future<Output = Result<Option<WorkspaceCheckout>>> + Send;

    fn remove_workspace(
        &self,
        workspace_id: &WorkspaceId,
    ) -> impl Future<Output = Result<()>> + Send;

    fn remove_checkout(&self, checkout_id: &CheckoutId) -> impl Future<Output = Result<()>> + Send;

    fn save_checkout(
        &self,
        checkout: WorkspaceCheckout,
    ) -> impl Future<Output = Result<WorkspaceCheckout>> + Send;

    fn refresh_checkout(
        &self,
        checkout_id: &CheckoutId,
    ) -> impl Future<Output = Result<Option<WorkspaceCheckout>>> + Send;
}
