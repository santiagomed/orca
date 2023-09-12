use crate::prompt::error::PromptTemplateError;

#[derive(Debug)]
pub enum ModelError {
    PromptTemplateError(PromptTemplateError),
    OpenAIError(async_openai::error::OpenAIError),
}

impl From<PromptTemplateError> for ModelError {
    fn from(err: PromptTemplateError) -> ModelError {
        ModelError::PromptTemplateError(err.into())
    }
}

impl From<async_openai::error::OpenAIError> for ModelError {
    fn from(err: async_openai::error::OpenAIError) -> ModelError {
        ModelError::OpenAIError(err)
    }
}
