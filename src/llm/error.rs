use crate::prompt::error::PromptTemplateError;

#[derive(Debug)]
pub enum LLMError {
    /// Prompt template error
    PromptTemplateError(PromptTemplateError),

    /// OpenAI error
    OpenAIError(async_openai::error::OpenAIError),

    /// NotImplemented error
    NotImplemented,
}

impl From<PromptTemplateError> for LLMError {
    /// Convert a prompt template error into an LLM error
    fn from(err: PromptTemplateError) -> LLMError {
        LLMError::PromptTemplateError(err.into())
    }
}

impl From<async_openai::error::OpenAIError> for LLMError {
    /// Convert an OpenAI error into an LLM error
    fn from(err: async_openai::error::OpenAIError) -> LLMError {
        LLMError::OpenAIError(err)
    }
}
