pub mod context;
use std::{any::Any, collections::HashMap, fmt::Display};

use serde;
use serde::Serialize;

use anyhow::Result;
use handlebars::Handlebars;

use chat::{remove_last_comma, ChatHelper, ChatPrompt, RoleHelper};

use crate::record::Record;

pub mod chat;

static SYSTEM_HELPER: RoleHelper = RoleHelper;
static USER_HELPER: RoleHelper = RoleHelper;
static ASSISTANT_HELPER: RoleHelper = RoleHelper;
static CHAT_HELPER: ChatHelper = ChatHelper;

/// Represents a prompt engine that uses handlebars templates to render strings.
pub struct TemplateEngine {
    /// The handlebars template engine
    reg: Handlebars<'static>,

    /// Registered templates
    pub templates: HashMap<String, String>,
}

impl Default for TemplateEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplateEngine {
    /// Creates a new `TemplateEngine` with the given prompt string.
    /// # Arguments
    /// * `prompt` - A string slice that holds the prompt template.
    ///
    /// # Example
    /// ```
    /// use orca_core::prompt::TemplateEngine;
    /// let prompt = TemplateEngine::new();
    /// ```
    pub fn new() -> TemplateEngine {
        let mut reg = Handlebars::new();
        reg.register_escape_fn(handlebars::no_escape);

        reg.register_helper("system", Box::new(SYSTEM_HELPER));
        reg.register_helper("user", Box::new(USER_HELPER));
        reg.register_helper("assistant", Box::new(ASSISTANT_HELPER));
        reg.register_helper("chat", Box::new(CHAT_HELPER));

        TemplateEngine {
            reg,
            templates: HashMap::new(),
        }
    }

    pub fn register_template(mut self, name: &str, template: &str) -> Result<Self> {
        self.templates.insert(name.to_string(), template.to_string());
        self.reg.register_template_string(name, template)?;
        Ok(self)
    }

    pub fn get_template(&self, name: &str) -> Option<String> {
        self.templates.get(name).cloned()
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
    /// use orca_core::prompt::TemplateEngine;
    ///
    /// let mut prompt = TemplateEngine::new().register_template("template", "Welcome!").unwrap();
    /// prompt.add_to_template("template", "Hello, world!");
    /// assert_eq!(prompt.templates["template"], "Welcome!Hello, world!");
    /// ```
    pub fn add_to_template(&mut self, name: &str, new_template: &str) {
        let mut chat = false;
        if let Some(template) = self.templates.get_mut(name) {
            if template.contains("{{#chat}}") && template.contains("{{/chat}}") {
                chat = true;
                *template = template.replace("{{#chat}}", "").replace("{{/chat}}", "");
            }
            template.push_str(new_template);
            if chat {
                *template = format!("{{{{#chat}}}}{}{{{{/chat}}}}", template);
            }
            self.reg.register_template_string(name, (*template).clone()).unwrap();
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
    /// use orca_core::prompt::TemplateEngine;
    ///
    /// let prompt = TemplateEngine::new().register_template("template", "{{#if true}}Hello, world!{{/if}}").unwrap();
    /// let result = prompt.render("template").unwrap();
    ///
    /// assert_eq!(result.to_string(), "Hello, world!".to_string());
    /// ```
    pub fn render(&self, name: &str) -> Result<Box<dyn Prompt>> {
        let rendered = self.reg.render(name, &HashMap::<String, String>::new())?;
        match serde_json::from_str::<ChatPrompt>(&rendered) {
            Ok(chat) => Ok(Box::new(chat)),
            Err(_) => Ok(Box::new(rendered)),
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
    /// use orca_core::prompt::TemplateEngine;
    ///
    /// let prompt = TemplateEngine::new().register_template("template", "Hello, {{name}}!").unwrap();
    /// let data = json!({"name": "world"});
    /// let result = prompt.render_context("template", &data).unwrap();
    ///
    /// assert_eq!(result.to_string(), "Hello, world!".to_string());
    /// ```
    pub fn render_context<T>(&self, template_name: &str, data: &T) -> Result<Box<dyn Prompt>>
    where
        T: Serialize,
    {
        let rendered = self.reg.render(template_name, data)?;
        log::info!("rendered: {}", rendered);
        // Check if rendered is a valid JSON string
        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&rendered) {
            match serde_json::from_value::<ChatPrompt>(json_value) {
                Ok(chat) => {
                    log::info!("Parsed as chat: {:?}", chat);
                    Ok(Box::new(chat))
                }
                Err(e) => {
                    log::info!("Failed to parse as chat, returning rendered string: {:?}", e);
                    Ok(Box::new(rendered))
                }
            }
        } else {
            // If rendered is not JSON, return it as a string
            log::info!("rendered is not JSON, returning as string");
            Ok(Box::new(rendered))
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
    /// # use serde_json::json;
    /// # use orca_core::prompt::TemplateEngine;
    /// # use orca_core::prompt::chat::{Role, Message, ChatPrompt};
    ///
    /// let prompt = TemplateEngine::new().register_template("template", "{{#system}}Hello, {{name}}!{{/system}}").unwrap();
    /// let data = json!({"name": "world"});
    /// let result = prompt.render_chat("template", Some(&data));
    /// ```
    pub fn render_chat<T>(&self, name: &str, data: Option<&T>) -> Result<ChatPrompt>
    where
        T: Serialize,
    {
        let rendered = match data {
            Some(data) => self.render_context(name, &data)?,
            None => self.render(name)?,
        };
        let rendered_json = format!("[{}]", remove_last_comma(&rendered.to_string()));
        let messages: ChatPrompt = serde_json::from_str(&rendered_json)?;
        Ok(messages)
    }
}

impl Clone for TemplateEngine {
    /// Clone a prompt template
    fn clone(&self) -> Self {
        TemplateEngine {
            reg: self.reg.clone(),
            templates: self.templates.clone(),
        }
    }
}

/// A trait representing a prompt for a Large Language Model.
///
/// The `Prompt` trait provides methods to transform, clone, save, and represent prompts in various formats.
pub trait Prompt: Sync + Send + Display {
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
    /// use orca_core::prompt;
    /// use orca_core::prompt::Prompt;
    ///
    /// let mut my_prompt = prompt!("Some prompt");
    /// let another_prompt = prompt!("Some other prompt");
    /// my_prompt.save(another_prompt);
    /// ```
    fn save(&mut self, _data: Box<dyn Prompt>) {
        unimplemented!("save not implemented for this prompt type");
    }

    /// Convert the current prompt to a `ChatPrompt`.
    ///
    /// # Returns
    /// * `Result<ChatPrompt>` - The `ChatPrompt` representation of the prompt or an error.
    fn to_chat(&self) -> Result<ChatPrompt> {
        unimplemented!("Unable to convert prompt to ChatPrompt");
    }

    /// Clone the current prompt into a Boxed trait object.
    ///
    /// # Returns
    ///
    /// * `Box<dyn Prompt>` - The cloned prompt.
    ///
    /// # Examples
    /// ```
    /// use orca_core::prompt;
    /// use orca_core::prompt::Prompt;
    ///
    /// let my_prompt = prompt!("Some prompt");
    /// let cloned_prompt = my_prompt.clone_prompt();
    /// ```
    fn clone_prompt(&self) -> Box<dyn Prompt>;

    fn as_any(&self) -> &dyn Any
    where
        Self: Sized + 'static,
    {
        self
    }
}

impl Prompt for ChatPrompt {
    fn save(&mut self, data: Box<dyn Prompt>) {
        let msgs = data.to_chat().unwrap();
        self.0.extend(msgs.0);
    }

    fn to_chat(&self) -> Result<ChatPrompt> {
        Ok(self.clone())
    }

    fn clone_prompt(&self) -> Box<dyn Prompt> {
        Box::new(self.clone())
    }
}

impl Prompt for String {
    fn save(&mut self, data: Box<dyn Prompt>) {
        self.push_str(&data.to_string());
    }

    fn clone_prompt(&self) -> Box<dyn Prompt> {
        Box::new(self.clone())
    }
}

impl Prompt for Record {
    fn clone_prompt(&self) -> Box<dyn Prompt> {
        Box::new(self.clone())
    }
}

#[macro_export]
macro_rules! prompt {
    ($e:expr) => {
        Box::new($e.to_string())
    };
}

#[macro_export]
/// takes in a vector or a series of prompts
macro_rules! prompts {
    ($e:expr) => {{
        $e
            .into_iter()
            .map(|x| Box::new(x.clone()) as Box<dyn crate::prompt::Prompt>)
            .collect::<Vec<Box<dyn crate::prompt::Prompt>>>()
    }};
    ($($e:expr),* $(,)?) => {
        {
            let mut prompts = Vec::new();
            $(
                prompts.push(Box::new($e.to_string()) as Box<dyn Prompt>);
            )*
            prompts
        }
    };
}

#[macro_export]
macro_rules! template {
    ($($name:expr, $template:expr),* $(,)?) => {
        {
            let mut engine = TemplateEngine::new();
            $(
                engine = engine.register_template($name, $template).unwrap();
            )*
            engine
        }
    };
}

#[cfg(test)]
mod test {

    use crate::prompt::chat::{Message, Role};
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn test_prompt() {
        let prompt_template = template!("my template", "What is the capital of {{country}}");
        let mut context = HashMap::new();
        context.insert("country", "France");
        let prompt = prompt_template.render_context("my template", &context).unwrap();
        assert_eq!(prompt.to_string(), "What is the capital of France");
    }

    #[test]
    fn test_chat() {
        let prompt_template = template!(
            "my template",
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
        let prompt = prompt_template.render_context("my template", &context).unwrap();
        assert_eq!(
            prompt.to_chat().unwrap(),
            ChatPrompt(vec![
                Message::new(Role::System, "You are NOT a master at math. You know nothing about it."),
                Message::new(Role::User, "What is your favorite aspect of math?"),
                Message::new(Role::Assistant, "I don't know anything about math."),
            ])
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
            "my template",
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

        let prompt = prompt_template.render_context("my template", &data).unwrap();
        assert_eq!(
            prompt.to_chat().unwrap(),
            ChatPrompt(vec![Message::new(
                Role::Assistant,
                "My name is gpt and I am 5 years old."
            )])
        );
    }
}
