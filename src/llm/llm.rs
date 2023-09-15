use super::error::LLMError;
use crate::prompt::{context::Context, prompt::PromptTemplate};
use serde::Serialize;

#[async_trait::async_trait(?Send)]
pub trait GenerateWithContext<T: Serialize> {
    async fn generate_with_context(&self, name: &str, context: &Context<T>, template: &PromptTemplate) -> Result<String, LLMError>;
}

#[async_trait::async_trait(?Send)]
pub trait GenerateWithData<T: Serialize> {
    async fn generate_with_data(&self, name: &str, data: &T, template: &PromptTemplate) -> Result<String, LLMError>;
}

pub trait LLM<T: Serialize>: GenerateWithContext<T> + GenerateWithData<T> {}
