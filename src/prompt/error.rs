/// Prompt template error
#[derive(Debug)]
pub enum PromptEngineError {
    /// Handlebars render error
    RenderError(handlebars::RenderError),

    /// Handlebars template error
    TemplateError(handlebars::TemplateError),
}

impl From<handlebars::RenderError> for PromptEngineError {
    /// Convert a handlebars render error into a prompt template error
    fn from(err: handlebars::RenderError) -> PromptEngineError {
        PromptEngineError::RenderError(err)
    }
}

impl From<handlebars::TemplateError> for PromptEngineError {
    /// Convert a handlebars template error into a prompt template error
    fn from(err: handlebars::TemplateError) -> PromptEngineError {
        PromptEngineError::TemplateError(err)
    }
}
