pub mod context;

use serde;
use serde::Serialize;
use std::fmt;
use std::fmt::{Display, Formatter};

use anyhow::Result;
use context::Context;
use handlebars::Handlebars;

#[derive(Serialize, PartialEq, Debug, Clone)]
pub struct Message {
    /// The message role (system, user, ai)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<Role>,

    /// The message text
    pub message: String,
}

impl Message {
    pub fn single(message: &str) -> Message {
        Message {
            role: None,
            message: message.to_string(),
        }
    }

    pub fn chat(role: Role, message: &str) -> Message {
        Message {
            role: Some(role),
            message: message.to_string(),
        }
    }

    pub fn into_vec(v: Vec<(&str, &str)>) -> Vec<Message> {
        let mut messages = Vec::new();
        for (role, message) in v {
            messages.push(Message::chat(role.into(), message));
        }
        messages
    }
}

impl Display for Message {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match &self.role {
            Some(role) => write!(f, "[{}] {}", role, self.message),
            None => write!(f, "{}", self.message),
        }
    }
}

#[derive(Debug, Serialize, Clone, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    #[default]
    User,
    Ai,
    Function,
}

/// Trait for converting a string to a role
impl From<&str> for Role {
    fn from(s: &str) -> Role {
        match s {
            "system" => Role::System,
            "user" => Role::User,
            "ai" => Role::Ai,
            _ => panic!("Invalid role: {}", s),
        }
    }
}

impl Display for Role {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Role::System => write!(f, "system"),
            Role::User => write!(f, "user"),
            Role::Ai => write!(f, "ai"),
            Role::Function => write!(f, "function"),
        }
    }
}

pub struct PromptEngine<'p> {
    /// A vector of template strings
    template: Vec<Message>,

    /// The handlebars template engine
    handlebars: Handlebars<'p>,
}

impl<'p> Default for PromptEngine<'p> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'p> PromptEngine<'p> {
    /// Initialize a prompt template
    /// # Example
    /// ```rust
    /// use orca::prompt::prompt::PromptEngine;
    ///
    /// let mut prompt_template = PromptEngine::new();
    /// ```
    pub fn new() -> PromptEngine<'p> {
        let mut handlebars = Handlebars::new();
        let template = Vec::new();
        handlebars.register_escape_fn(handlebars::no_escape);

        PromptEngine { template, handlebars }
    }

    /// Initialize a prompt template with a single template string
    /// If chat format is desired, use from_chat instead
    /// # Example
    /// ```rust
    /// use orca::prompt::prompt::PromptEngine;
    ///
    /// let mut prompt_template = PromptEngine::new().from_prompt("What is the capital of {{country}}");
    /// ```
    pub fn from_prompt(mut self, template: &str) -> PromptEngine<'p> {
        self.template = vec![Message::single(template)];
        self
    }

    /// Initialize a prompt template with multiple template strings
    /// If single format is desired, use from_prompt instead
    /// # Example
    /// ```rust
    /// use orca::prompt::prompt::PromptEngine;
    ///
    /// let mut prompt_template = PromptEngine::new().from_chat(vec![
    ///    ("system", "You are NOT a master at {{subject}}. You know nothing about it."),
    ///    ("user", "What is your favorite aspect of {{subject}}?"),
    ///    ("ai", "I don't know anything about {{subject}}."),
    /// ]);
    /// ```
    pub fn from_chat(mut self, templates: Vec<(&str, &str)>) -> PromptEngine<'p> {
        self.template = Message::into_vec(templates);
        self
    }

    /// Add a new template string to the prompt template
    /// # Example
    /// ```rust
    /// use orca::prompt::prompt::PromptEngine;
    /// use orca::prompt::Message;
    ///
    /// let mut prompt_template = PromptEngine::new().from_prompt("What is the capital of {{country}}");
    /// prompt_template.add_prompt(("ai", "The capital is {{capital}}"));
    /// ```
    pub fn add_prompt(&mut self, template: (&str, &str)) {
        self.template.push(Message::chat(template.0.into(), template.1));
    }

    /// Render a prompt template
    /// # Example
    /// ```rust
    /// use orca::prompt::{prompt::PromptEngine, context::Context};
    /// use orca::prompt::{Message, Role};
    ///
    /// let mut prompt_template = PromptEngine::new().from_prompt("Your name is {{name}}");
    /// let mut context = Context::new();
    /// context.set("name", "gpt");
    /// let prompt = prompt_template.render_context(&context).unwrap();
    /// assert_eq!(prompt, vec![Message::single("Your name is gpt")]);
    /// ```
    pub fn render_context<T>(&self, context: &Context<T>) -> Result<Vec<Message>>
    where
        T: Serialize,
    {
        let mut messages = Vec::new();
        for message in &self.template {
            let rendered = self.handlebars.render_template(&message.message, &context.get_variables())?;
            messages.push(Message {
                role: message.role.clone(),
                message: rendered,
            });
        }
        Ok(messages)
    }

    /// Render a prompt template with data
    /// # Example
    /// ```rust
    /// use orca::prompt::{prompt::{PromptEngine}, context::Context};
    /// use orca::prompt::{Message, Role};
    /// use serde::Serialize;
    ///
    /// #[derive(Serialize)]
    /// struct Data {
    ///    name: String,
    ///    age: u8,
    /// }
    ///
    /// let mut prompt_template = PromptEngine::new().from_chat(vec![
    ///   ("ai", "My name is {{name}} and I am {{#if (eq age 1)}}1 year{{else}}{{age}} years{{/if}} old."),
    /// ]);
    ///
    /// let data = Data {
    ///   name: "gpt".to_string(),
    ///   age: 5,
    /// };
    ///
    /// let prompt = prompt_template.render(&data).unwrap();
    /// assert_eq!(prompt, vec![Message::chat(Role::Ai, "My name is gpt and I am 5 years old.")]);
    /// ```
    pub fn render<T>(&self, data: &T) -> Result<Vec<Message>>
    where
        T: Serialize,
    {
        let mut messages = Vec::new();
        for message in &self.template {
            let rendered = self.handlebars.render_template(&message.message, &data)?;
            messages.push(Message {
                role: message.role.clone(),
                message: rendered,
            });
        }
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
        PromptEngine::new().from_prompt($template)
    };
}

#[macro_export]
macro_rules! prompts {
    ($($template:expr),+) => {
        PromptEngine::new().from_chat(vec![$($template),+])
    };
}

#[cfg(test)]
mod test {
    use crate::prompt::Role;

    use super::*;

    #[test]
    fn test_prompt() {
        let prompt_template = prompt!("What is the capital of {{country}}");
        let mut context = Context::new();
        context.set("country", "France");
        let prompt = prompt_template.render_context(&context).unwrap();
        assert_eq!(prompt, vec![Message::single("What is the capital of France")]);
    }

    #[test]
    fn test_chat() {
        let prompt_template = prompts!(
            (
                "system",
                "You are NOT a master at {{subject}}. You know nothing about it."
            ),
            ("user", "What is your favorite aspect of {{subject}}?"),
            ("ai", "I don't know anything about {{subject}}.")
        );
        let mut context = Context::new();
        context.set("subject", "math");
        let prompt = prompt_template.render_context(&context).unwrap();
        assert_eq!(
            prompt,
            vec![
                Message::chat(Role::System, "You are NOT a master at math. You know nothing about it."),
                Message::chat(Role::User, "What is your favorite aspect of math?"),
                Message::chat(Role::Ai, "I don't know anything about math."),
            ]
        );
    }

    #[test]
    fn test_context() {
        let prompt_template = prompts!(("system", "This is my data: {{data}}."));

        let mut context = Context::new();
        context.set(
            "data",
            serde_json::json!({"name": "gpt", "age": 5, "country": "France"}),
        );
        let prompt = prompt_template.render_context(&context).unwrap();
        assert_eq!(
            prompt,
            vec![Message::chat(
                Role::System,
                "This is my data: {age:5,country:France,name:gpt}."
            )]
        );
    }

    #[test]
    fn test_data() {
        #[derive(Serialize)]
        struct Data {
            name: String,
            age: u8,
        }

        let prompt_template = prompts!((
            "ai",
            "My name is {{name}} and I am {{#if (eq age 1)}}1 year{{else}}{{age}} years{{/if}} old.",
        ));

        let data = Data {
            name: "gpt".to_string(),
            age: 5,
        };

        let prompt = prompt_template.render(&data).unwrap();
        assert_eq!(
            prompt,
            vec![Message::chat(Role::Ai, "My name is gpt and I am 5 years old.")]
        );
    }
}
