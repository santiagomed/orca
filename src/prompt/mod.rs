pub mod context;
use std::collections::HashMap;

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
pub struct TemplateEngine<'p> {
    /// A vector of template strings
    pub template: String,

    /// The handlebars template engine
    handlebars: Handlebars<'p>,
}

impl<'p> TemplateEngine<'p> {
    /// Creates a new `TemplateEngine` with the given prompt string.
    /// # Arguments
    /// * `prompt` - A string slice that holds the prompt template.
    ///
    /// # Example
    /// ```
    /// use orca::prompt::TemplateEngine;
    /// let prompt = TemplateEngine::new("Welcome, {{user}}!");
    /// ```
    pub fn new(prompt: &str) -> TemplateEngine<'p> {
        let mut handlebars = Handlebars::new();
        let template = prompt.to_string();
        handlebars.register_escape_fn(handlebars::no_escape);

        handlebars.register_helper("system", Box::new(SYSTEM_HELPER));
        handlebars.register_helper("user", Box::new(USER_HELPER));
        handlebars.register_helper("assistant", Box::new(ASSISTANT_HELPER));
        handlebars.register_helper("chat", Box::new(CHAT_HELPER));

        TemplateEngine { template, handlebars }
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
    /// use orca::prompt::TemplateEngine;
    ///
    /// let mut prompt = TemplateEngine::new("Welcome!");
    /// prompt.add_to_template("Hello, world!");
    /// assert_eq!(prompt.template, "Welcome!Hello, world!");
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

    /// Renders a Handlebars template and returns the result as a Boxed trait object.
    ///
    /// # Arguments
    /// * `data` - A reference to the data to be used in the template rendering.
    ///
    /// # Returns
    /// Returns a Boxed trait object that implements the `Prompt` trait.
    ///
    /// # Example
    /// ```
    /// use serde_json::json;
    /// use orca::prompt::TemplateEngine;
    ///
    /// let prompt = TemplateEngine::new("{{#if true}}Hello, world!{{/if}}");
    /// let result = prompt.render().unwrap();
    ///
    /// assert_eq!(result.to_string().unwrap(), "Hello, world!".to_string());
    /// ```
    pub fn render(&self) -> Result<Box<dyn Prompt>> {
        let rendered = self.handlebars.render_template(&self.template, &HashMap::<String, String>::new())?;
        match serde_json::from_str::<ChatPrompt>(&rendered) {
            Ok(chat) => return Ok(Box::new(chat)),
            Err(_) => return Ok(Box::new(rendered)),
        }
    }

    /// Renders a Handlebars template with the given data and returns the result as a Boxed trait object.
    ///
    /// # Arguments
    /// * `data` - A reference to the data to be used in the template rendering.
    ///
    /// # Returns
    /// Returns a `Result` containing the rendered template as a Boxed trait object.
    ///
    /// # Example
    /// ```
    /// use serde_json::json;
    /// use orca::prompt::TemplateEngine;
    ///
    /// let prompt = TemplateEngine::new("Hello, {{name}}!");
    /// let data = json!({"name": "world"});
    /// let result = prompt.render_context(&data).unwrap();
    ///
    /// assert_eq!(result.to_string().unwrap(), "Hello, world!".to_string());
    /// ```
    pub fn render_context<T>(&self, data: &T) -> Result<Box<dyn Prompt>>
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
    /// use orca::prompt::TemplateEngine;
    /// use orca::prompt::chat::{Role, Message};
    /// use async_openai::types::Role as R;
    ///
    /// let prompt = TemplateEngine::new("{{#system}}Hello, {{name}}!{{/system}}");
    /// let data = json!({"name": "world"});
    /// let result = prompt.render_chat(Some(&data));
    /// assert_eq!(result.unwrap(), vec![Message::new(Role(R::System), "Hello, world!")]);
    /// ```
    pub fn render_chat<T>(&self, data: Option<&T>) -> Result<ChatPrompt>
    where
        T: Serialize,
    {
        let rendered = match data {
            Some(data) => self.render_context(&data)?,
            None => self.render()?,
        };
        let rendered_json = format!("[{}]", remove_last_comma(rendered.as_str()?));
        let messages: ChatPrompt = serde_json::from_str(&rendered_json)?;
        Ok(messages)
    }
}

impl<'p> Clone for TemplateEngine<'p> {
    /// Clone a prompt template
    fn clone(&self) -> Self {
        TemplateEngine {
            template: self.template.clone(),
            handlebars: self.handlebars.clone(),
        }
    }
}

/// A trait representing a prompt for a Large Language Model.
///
/// The `Prompt` trait provides methods to transform, clone, save, and represent prompts in various formats.
pub trait Prompt {
    /// Save the data from another `Prompt` into the current one.
    ///
    /// # Arguments
    /// * `data` - A boxed trait object implementing the `Prompt` trait.
    ///
    /// # Returns
    /// * `Result<()>` - An empty result indicating success or an error.
    ///
    /// # Examples
    /// ```
    /// use orca::prompt;
    /// use orca::prompt::Prompt;
    ///
    /// let mut my_prompt = prompt!("Some prompt");
    /// let another_prompt = prompt!("Some other prompt");
    /// my_prompt.save(another_prompt).unwrap();
    /// ```
    fn save(&mut self, data: Box<dyn Prompt>) -> Result<()>;

    /// Convert the current prompt to a `String`.
    ///
    /// # Returns
    /// * `Result<String>` - The `String` representation of the prompt or an error.
    ///
    /// # Examples
    /// ```
    /// use orca::prompt;
    /// use orca::prompt::Prompt;
    ///
    /// let my_prompt = prompt!("Some prompt");
    /// assert_eq!(my_prompt.to_string(), "Some prompt".to_string());
    /// ```
    fn to_string(&self) -> Result<String>;

    /// Get the current prompt as a string slice.
    ///
    /// # Returns
    /// * `Result<&str>` - The string slice representation of the prompt or an error.
    fn as_str(&self) -> Result<&str>;

    /// Convert the current prompt to a `ChatPrompt`.
    ///
    /// # Returns
    /// * `Result<ChatPrompt>` - The `ChatPrompt` representation of the prompt or an error.
    fn to_chat(&self) -> Result<ChatPrompt>;

    /// Clone the current prompt into a Boxed trait object.
    ///
    /// # Returns
    ///
    /// * `Box<dyn Prompt>` - The cloned prompt.
    ///
    /// # Examples
    /// ```
    /// use orca::prompt;
    /// use orca::prompt::Prompt;
    ///
    /// let my_prompt = prompt!("Some prompt");
    /// let cloned_prompt = my_prompt.clone_prompt();
    /// ```
    fn clone_prompt(&self) -> Box<dyn Prompt>;
}

impl Prompt for ChatPrompt {
    fn save(&mut self, data: Box<dyn Prompt>) -> Result<()> {
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

    fn clone_prompt(&self) -> Box<dyn Prompt> {
        Box::new(self.clone())
    }
}
impl Prompt for String {
    fn save(&mut self, data: Box<dyn Prompt>) -> Result<()> {
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

    fn clone_prompt(&self) -> Box<dyn Prompt> {
        Box::new(self.clone())
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
    ($e:expr) => {
        Box::new($e.to_string())
    };
}

#[macro_export]
macro_rules! template {
    ($template:expr) => {
        TemplateEngine::new($template)
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
        let prompt_template = template!("What is the capital of {{country}}");
        let mut context = HashMap::new();
        context.insert("country", "France");
        let prompt = prompt_template.render_context(&context).unwrap();
        assert_eq!(prompt.to_string().unwrap(), "What is the capital of France");
    }

    #[test]
    fn test_chat() {
        let prompt_template = template!(
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
        let prompt = prompt_template.render_context(&context).unwrap();
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

        let prompt_template = template!(
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

        let prompt = prompt_template.render_context(&data).unwrap();
        assert_eq!(
            prompt.to_chat().unwrap(),
            vec![Message::new(Role(R::Assistant), "My name is gpt and I am 5 years old.")]
        );
    }
}
