use crate::prompt;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LLMError {
    #[error(transparent)]
    PromptEngineError(#[from] prompt::PromptEngineError),

    #[error(transparent)]
    OpenAIError(#[from] async_openai::error::OpenAIError),

    #[error("Functionality not implemented")]
    NotImplemented,
}
