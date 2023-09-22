use crate::prompt::error::PromptEngineError;

#[derive(Debug)]
pub enum LLMError {
    /// Prompt template error
    PromptEngineError(PromptEngineError),

    /// OpenAI error
    OpenAIError(async_openai::error::OpenAIError),

    /// NotImplemented error
    NotImplemented,
}

impl From<PromptEngineError> for LLMError {
    /// Convert a prompt template error into an LLM error
    fn from(err: PromptEngineError) -> LLMError {
        LLMError::PromptEngineError(err.into())
    }
}

impl From<async_openai::error::OpenAIError> for LLMError {
    /// Convert an OpenAI error into an LLM error
    fn from(err: async_openai::error::OpenAIError) -> LLMError {
        LLMError::OpenAIError(err)
    }
}
