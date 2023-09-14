use super::error::LLMError;
use crate::prompt::{context::Context, prompt::PromptTemplate};
use serde::Serialize;

#[async_trait::async_trait]
pub trait GenerateWithContext<T: Serialize + std::marker::Sync + std::fmt::Display> {
    async fn generate_with_context(&self, name: &str, context: &Context<T>, template: &PromptTemplate) -> Result<String, LLMError>;
}

#[async_trait::async_trait]
pub trait GenerateWithData<T: Serialize + std::marker::Sync + std::fmt::Display> {
    async fn generate_with_data(&self, name: &str, data: &T, template: &PromptTemplate) -> Result<String, LLMError>;
}

pub trait LLM<T: Serialize + std::marker::Sync + std::fmt::Display>: GenerateWithContext<T> + GenerateWithData<T> {}
