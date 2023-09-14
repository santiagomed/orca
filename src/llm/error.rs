use crate::prompt::error::PromptTemplateError;

#[derive(Debug)]
pub enum LLMError {
    PromptTemplateError(PromptTemplateError),
    OpenAIError(async_openai::error::OpenAIError),
    NotImplemented,
}

impl From<PromptTemplateError> for LLMError {
    fn from(err: PromptTemplateError) -> LLMError {
        LLMError::PromptTemplateError(err.into())
    }
}

impl From<async_openai::error::OpenAIError> for LLMError {
    fn from(err: async_openai::error::OpenAIError) -> LLMError {
        LLMError::OpenAIError(err)
    }
}
