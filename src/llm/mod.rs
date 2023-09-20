pub mod error;
pub mod openai;

use error::LLMError;
use crate::prompt::prompt::Message;

/// Generate with context trait is used to execute an LLM using a context and a prompt template.
/// The context is a previously created context using the Context struct. The prompt template
/// is a previously created prompt template using the prompt! macro.
#[async_trait::async_trait(?Send)]
pub trait Generate {
    /// Generate a response from an LLM using a context and a prompt template.
    async fn generate(&self, prompt: &Vec<Message>) -> Result<String, LLMError>;
}
