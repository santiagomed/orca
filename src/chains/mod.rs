pub mod chain;
pub mod sequential;

use crate::llm::error::LLMError;

#[async_trait::async_trait(?Send)]
pub trait Execute<T> {
    /// Execute an LLM chain using a context and a prompt template.
    async fn execute(&mut self, data: &T) -> Result<String, LLMError>;
}
