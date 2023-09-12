/// Prompt template error
#[derive(Debug)]
pub enum PromptTemplateError {
    /// Handlebars render error
    RenderError(handlebars::RenderError),

    /// Handlebars template error
    TemplateError(handlebars::TemplateError),
}

impl From<handlebars::RenderError> for PromptTemplateError {
    fn from(err: handlebars::RenderError) -> PromptTemplateError {
        PromptTemplateError::RenderError(err)
    }
}

impl From<handlebars::TemplateError> for PromptTemplateError {
    fn from(err: handlebars::TemplateError) -> PromptTemplateError {
        PromptTemplateError::TemplateError(err)
    }
}
