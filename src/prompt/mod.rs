pub mod context;
use serde;
use serde::Serialize;

use anyhow::Result;
use chat::Message;
use handlebars::Handlebars;

use self::chat::RoleHelper;

pub mod chat;

static SYSTEM_HELPER: RoleHelper = RoleHelper;
static USER_HELPER: RoleHelper = RoleHelper;
static ASSISTANT_HELPER: RoleHelper = RoleHelper;

/// Represents a prompt engine that uses handlebars templates to render strings.
pub struct PromptEngine<'p> {
    /// A vector of template strings
    pub template: String,

    /// The handlebars template engine
    handlebars: Handlebars<'p>,
}

impl<'p> PromptEngine<'p> {
    /// Creates a new `PromptEngine` with the given prompt string.
    /// # Arguments
    /// * `prompt` - A string slice that holds the prompt template.
    ///
    /// # Example
    /// ```
    /// use orca::prompt::PromptEngine;
    /// let prompt = PromptEngine::new("Welcome, {{user}}!");
    /// ```
    pub fn new(prompt: &str) -> PromptEngine<'p> {
        let mut handlebars = Handlebars::new();
        let template = prompt.to_string();
        handlebars.register_escape_fn(handlebars::no_escape);

        handlebars.register_helper("system", Box::new(SYSTEM_HELPER));
        handlebars.register_helper("user", Box::new(USER_HELPER));
        handlebars.register_helper("assistant", Box::new(ASSISTANT_HELPER));

        PromptEngine { template, handlebars }
    }

    /// Adds a new template to the prompt.
    ///
    /// This function appends a new template to the existing prompt. The template
    /// should be a string slice that holds the template to be added. The template
    /// will be added to the prompt on a new line.
    ///
    /// # Arguments
    /// * `template` - A string slice that holds the template to be added.
    ///
    /// # Example
    /// ```
    /// use orca::prompt::PromptEngine;
    ///
    /// let mut prompt = PromptEngine::new("Welcome!");
    /// prompt.add_to_prompt("Hello, world!");
    /// assert_eq!(prompt.template, "Welcome!\nHello, world!");
    /// ```
    pub fn add_to_prompt(&mut self, template: &str) {
        self.template.push_str(format!("\n{}", template).as_str());
    }

    /// Renders a Handlebars template with the given data and returns the result as a String.
    ///
    /// # Arguments
    /// * `data` - A reference to the data to be used in the template rendering.
    ///
    /// # Returns
    /// Returns a `Result` containing the rendered template as a `String` if successful, or an error if the rendering fails.
    ///
    /// # Example
    /// ```
    /// use serde_json::json;
    /// use orca::prompt::PromptEngine;
    /// use async_openai::types::Role as R;
    ///
    /// let prompt = PromptEngine::new("Hello, {{name}}!");
    /// let data = json!({"name": "world"});
    /// let result = prompt.render(&data);
    ///
    /// assert_eq!(result.unwrap(), "Hello, world!".to_string());
    /// ```
    pub fn render<T>(&self, data: &T) -> Result<String>
    where
        T: Serialize,
    {
        let rendered = self.handlebars.render_template(&self.template, &data)?;
        Ok(rendered)
    }

    /// Renders a Handlebars template with the given data and returns the result as a vector of `Message`s.
    ///
    /// # Arguments
    /// * `data` - A reference to the data to be used in the template rendering.
    ///
    /// # Returns
    /// Returns a `Result` containing the rendered template as a vector of `Message`s if successful, or an error if the rendering fails.
    ///
    /// # Example
    /// ```
    /// use serde_json::json;
    /// use orca::prompt::{PromptEngine};
    /// use orca::prompt::chat::{Role, Message};
    /// use async_openai::types::Role as R;
    ///
    /// let prompt = PromptEngine::new("{{#system}}Hello, {{name}}!{{/system}}");
    /// let data = json!({"name": "world"});
    /// let result = prompt.render_chat(&data);
    /// assert_eq!(result.unwrap(), vec![Message::new(Role(R::System), "Hello, world!")]);
    /// ```
    pub fn render_chat<T>(&self, data: &T) -> Result<Vec<Message>>
    where
        T: Serialize,
    {
        let rendered = self.render(data)?;
        let rendered_json = format!("[{}]", rendered.trim().trim_end_matches(','));
        let messages: Vec<Message> = serde_json::from_str(&rendered_json)?;
        Ok(messages)
    }
}

impl<'p> Clone for PromptEngine<'p> {
    /// Clone a prompt template
    fn clone(&self) -> Self {
        PromptEngine {
            template: self.template.clone(),
            handlebars: self.handlebars.clone(),
        }
    }
}

#[macro_export]
macro_rules! prompt {
    ($template:expr) => {
        PromptEngine::new($template)
    };
}

#[cfg(test)]
mod test {

    use crate::prompt::chat::Role;
    use async_openai::types::Role as R;
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn test_prompt() {
        let prompt_template = prompt!("What is the capital of {{country}}");
        let mut context = HashMap::new();
        context.insert("country", "France");
        let prompt = prompt_template.render(&context).unwrap();
        assert_eq!(prompt, "What is the capital of France");
    }

    #[test]
    fn test_chat() {
        let prompt_template = prompt!(
            r#"
                {{#system}}
                You are NOT a master at {{subject}}. You know nothing about it.
                {{/system}}
                {{#user}}
                What is your favorite aspect of {{subject}}?
                {{/user}}
                {{#assistant}}
                I don't know anything about {{subject}}.
                {{/assistant}}
            "#
        );
        let mut context = HashMap::new();
        context.insert("subject", "math");
        let prompt = prompt_template.render_chat(&context).unwrap();
        assert_eq!(
            prompt,
            vec![
                Message::new(
                    Role(R::System),
                    "You are NOT a master at math. You know nothing about it."
                ),
                Message::new(Role(R::User), "What is your favorite aspect of math?"),
                Message::new(Role(R::Assistant), "I don't know anything about math."),
            ]
        );
    }

    #[test]
    fn test_data() {
        #[derive(Serialize)]
        struct Data {
            name: String,
            age: u8,
        }

        let prompt_template = prompt!(
            "{{#assistant}}
            My name is {{name}} and I am {{#if (eq age 1)}}1 year{{else}}{{age}} years{{/if}} old.
            {{/assistant}}"
        );

        let data = Data {
            name: "gpt".to_string(),
            age: 5,
        };

        let prompt = prompt_template.render_chat(&data).unwrap();
        assert_eq!(
            prompt,
            vec![Message::new(Role(R::Assistant), "My name is gpt and I am 5 years old.")]
        );
    }
}
