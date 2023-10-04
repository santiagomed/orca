pub mod context;
use serde;
use serde::Serialize;

use anyhow::Result;
use handlebars::Handlebars;

use chat::{remove_last_comma, ChatHelper, ChatPrompt, RoleHelper};

pub mod chat;

static SYSTEM_HELPER: RoleHelper = RoleHelper;
static USER_HELPER: RoleHelper = RoleHelper;
static ASSISTANT_HELPER: RoleHelper = RoleHelper;
static CHAT_HELPER: ChatHelper = ChatHelper;

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
        handlebars.register_helper("chat", Box::new(CHAT_HELPER));

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
    pub fn add_to_template(&mut self, template: &str) {
        // TODO: Remove this temporary hack to add a new prompt to template
        //       where we remove the chat tags from the template and add them
        //       back after appending the new template.
        let mut chat = false;
        if self.template.contains("{{#chat}}") && self.template.contains("{{/chat}}") {
            chat = true;
            self.template = self.template.replace("{{#chat}}", "").replace("{{/chat}}", "");
        }
        self.template.push_str(template);
        if chat {
            self.template = format!("{{{{#chat}}}}{}{{{{/chat}}}}", self.template);
        }
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
    pub fn render<T>(&self, data: &T) -> Result<Box<dyn Prompt>>
    where
        T: Serialize,
    {
        let rendered = self.handlebars.render_template(&self.template, &data)?;
        match serde_json::from_str::<ChatPrompt>(&rendered) {
            Ok(chat) => return Ok(Box::new(chat)),
            Err(_) => return Ok(Box::new(rendered)),
        }
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
    pub fn render_chat<T>(&self, data: &T) -> Result<ChatPrompt>
    where
        T: Serialize,
    {
        let rendered = self.render(data)?;
        let rendered_json = format!("[{}]", remove_last_comma(rendered.as_str()?));
        let messages: ChatPrompt = serde_json::from_str(&rendered_json)?;
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

pub trait Prompt {
    fn save(&mut self, data: &dyn Prompt) -> Result<()>;
    fn to_string(&self) -> Result<String>;
    fn as_str(&self) -> Result<&str>;
    fn to_chat(&self) -> Result<ChatPrompt>;
}

impl Prompt for ChatPrompt {
    fn save(&mut self, data: &dyn Prompt) -> Result<()> {
        // let msgs = serde_json::from_str::<ChatPrompt>(&format!("[{}]", &remove_last_comma(data)))?;
        let msgs = data.to_chat()?;
        self.extend(msgs);
        Ok(())
    }

    fn to_string(&self) -> Result<String> {
        Ok(serde_json::to_string(self)?)
    }

    fn as_str(&self) -> Result<&str> {
        Err(anyhow::anyhow!("Unable to convert ChatPrompt to &str"))
    }

    fn to_chat(&self) -> Result<ChatPrompt> {
        Ok(self.clone())
    }
}
impl Prompt for String {
    fn save(&mut self, data: &dyn Prompt) -> Result<()> {
        self.push_str(data.as_str()?);
        Ok(())
    }

    fn to_string(&self) -> Result<String> {
        Ok(self.clone())
    }

    fn as_str(&self) -> Result<&str> {
        Ok(self.as_str())
    }

    fn to_chat(&self) -> Result<ChatPrompt> {
        Err(anyhow::anyhow!("Unable to convert String to ChatPrompt"))
    }
}

/// Cleans the prompt by removing unparsable characters and quotations.
pub fn clean_prompt(content: &str, quotes: bool) -> String {
    content
        .chars()
        .filter(|&c| c > '\u{1F}')
        .filter(|&c| if quotes { c != '"' } else { true })
        .collect::<String>()
        .replace("&nbsp;", " ")
}

#[macro_export]
macro_rules! prompt {
    ($template:expr) => {
        PromptEngine::new($template)
    };
}

#[cfg(test)]
mod test {

    use crate::prompt::chat::{Message, Role};
    use async_openai::types::Role as R;
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn test_prompt() {
        let prompt_template = prompt!("What is the capital of {{country}}");
        let mut context = HashMap::new();
        context.insert("country", "France");
        let prompt = prompt_template.render(&context).unwrap();
        assert_eq!(prompt.to_string().unwrap(), "What is the capital of France");
    }

    #[test]
    fn test_chat() {
        let prompt_template = prompt!(
            r#"
                {{#chat}}
                {{#system}}
                You are NOT a master at {{subject}}. You know nothing about it.
                {{/system}}
                {{#user}}
                What is your favorite aspect of {{subject}}?
                {{/user}}
                {{#assistant}}
                I don't know anything about {{subject}}.
                {{/assistant}}
                {{/chat}}
            "#
        );
        let mut context = HashMap::new();
        context.insert("subject", "math");
        let prompt = prompt_template.render(&context).unwrap();
        assert_eq!(
            prompt.to_chat().unwrap(),
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
            "{{#chat}}
            {{#assistant}}
            My name is {{name}} and I am {{#if (eq age 1)}}1 year{{else}}{{age}} years{{/if}} old.
            {{/assistant}}
            {{/chat}}"
        );

        let data = Data {
            name: "gpt".to_string(),
            age: 5,
        };

        let prompt = prompt_template.render(&data).unwrap();
        assert_eq!(
            prompt.to_chat().unwrap(),
            vec![Message::new(Role(R::Assistant), "My name is gpt and I am 5 years old.")]
        );
    }
}
