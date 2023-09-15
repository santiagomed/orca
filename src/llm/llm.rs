use super::error::LLMError;
use crate::prompt::{context::Context, prompt::PromptTemplate};
use serde::Serialize;

/// Generate with context trait is used to execute an LLM using a context and a prompt template.
/// The context is a previously created context using the Context struct. The prompt template
/// is a previously created prompt template using the prompt! macro.
#[async_trait::async_trait(?Send)]
pub trait GenerateWithContext<T: Serialize> {
    /// Generate a response from an LLM using a context and a prompt template.
    async fn generate_with_context(&self, name: &str, context: &Context<T>, template: &PromptTemplate) -> Result<String, LLMError>;
}

/// Generate with data trait is used to execute an LLM using a data structure and a prompt template.
/// The data structure can be any struct that implements the Serialize trait. The prompt template
/// is a previously created prompt template using the prompt! macro.
#[async_trait::async_trait(?Send)]
pub trait GenerateWithData<T: Serialize> {
    /// Generate a response from an LLM using a data structure and a prompt template.
    async fn generate_with_data(&self, name: &str, data: &T, template: &PromptTemplate) -> Result<String, LLMError>;
}

/// LLM trait is used to implement an LLM the generate traits for any type that needs to implement
/// both GenerateWithContext and GenerateWithData. This applies to any LLM type as well as
/// LLMChains.
pub trait LLM<T: Serialize>: GenerateWithContext<T> + GenerateWithData<T> {}
