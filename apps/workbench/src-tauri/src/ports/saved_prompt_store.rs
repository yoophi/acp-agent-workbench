use anyhow::Result;
use std::future::Future;

use crate::domain::saved_prompt::{
    CreateSavedPromptInput, SavedPrompt, SavedPromptId, UpdateSavedPromptPatch,
};

pub trait SavedPromptStore: Clone + Send + Sync + 'static {
    fn list_saved_prompts(
        &self,
        workspace_id: Option<&str>,
    ) -> impl Future<Output = Result<Vec<SavedPrompt>>> + Send;

    fn create_saved_prompt(
        &self,
        input: CreateSavedPromptInput,
    ) -> impl Future<Output = Result<SavedPrompt>> + Send;

    fn update_saved_prompt(
        &self,
        id: &SavedPromptId,
        patch: UpdateSavedPromptPatch,
    ) -> impl Future<Output = Result<Option<SavedPrompt>>> + Send;

    fn delete_saved_prompt(&self, id: &SavedPromptId) -> impl Future<Output = Result<()>> + Send;

    fn record_saved_prompt_used(
        &self,
        id: &SavedPromptId,
    ) -> impl Future<Output = Result<Option<SavedPrompt>>> + Send;
}
