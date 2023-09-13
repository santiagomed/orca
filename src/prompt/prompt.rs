use std::collections::HashMap;

use super::{context::Context, error::PromptTemplateError};
use handlebars::Handlebars;
use serde::Serialize;

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

pub struct PromptTemplate<'a> {
    /// A map of template names to template strings
    templates: HashMap<String, Vec<Message>>,

    /// The handlebars template engine
    handlebars: Handlebars<'a>,
}

impl<'a> PromptTemplate<'a> {
    /// Initialize a prompt template
    /// # Example
    /// ```rust
    /// use orcha::prompt::prompt::PromptTemplate;
    ///
    /// let mut prompt_template = PromptTemplate::new();
    /// ```
    pub fn new() -> PromptTemplate<'a> {
        let mut handlebars = Handlebars::new();
        let templates = HashMap::new();
        handlebars.register_escape_fn(handlebars::no_escape);

        PromptTemplate {
            templates,
            handlebars,
        }
    }

    /// Initialize a prompt template with a single template string
    /// If chat format is desired, use from_chat instead
    /// # Example
    /// ```rust
    /// use orcha::prompt::prompt::PromptTemplate;
    ///
    /// let mut prompt_template = PromptTemplate::new().from_prompt("prompt", "What is the capital of {{country}}");
    /// ```
    pub fn from_prompt(mut self, name: &str, template: &str) -> PromptTemplate<'a> {
        self.templates
            .insert(name.to_string(), vec![Message::single(template)]);
        self
    }

    /// Initialize a prompt template with multiple template strings
    /// If single format is desired, use from_prompt instead
    /// # Example
    /// ```rust
    /// use orcha::prompt::prompt::PromptTemplate;
    ///
    /// let mut prompt_template = PromptTemplate::new().from_chat("prompt", vec![
    ///    ("system", "You are NOT a master at {{subject}}. You know nothing about it."),
    ///    ("user", "What is your favorite aspect of {{subject}}?"),
    ///    ("ai", "I don't know anything about {{subject}}."),
    /// ]);
    /// ```
    pub fn from_chat(mut self, name: &str, templates: Vec<(&str, &str)>) -> PromptTemplate<'a> {
        self.templates
            .insert(name.to_string(), Message::into_vec(templates));
        self
    }

    /// Render a prompt template
    /// # Example
    /// ```rust
    /// use orcha::prompt::{prompt::{Message, PromptTemplate}, context::Context};
    ///
    /// let mut prompt_template = PromptTemplate::new().from_prompt("prompt", "Your name is {{name}}");
    /// let mut context = Context::new();
    /// context.set("name", "gpt");
    /// let prompt = prompt_template.render("prompt", &context).unwrap();
    /// assert_eq!(prompt, vec![Message::single("Your name is gpt")]);
    /// ```
    pub fn render_context<T: Serialize + std::fmt::Display>(
        &self,
        name: &str,
        context: &Context<T>,
    ) -> Result<Vec<Message>, PromptTemplateError> {
        let template = self.templates.get(name).unwrap();
        let mut messages = Vec::new();
        for message in template {
            let rendered = self
                .handlebars
                .render_template(&message.message, &context.get_variables())?;
            messages.push(Message {
                role: message.role.clone(),
                message: rendered,
            });
        }
        Ok(messages)
    }

    pub fn render_data<K>(&self, name: &str, data: K) -> Result<Vec<Message>, PromptTemplateError>
    where
        K: Serialize,
    {
        let template = self.templates.get(name).unwrap();
        let mut messages = Vec::new();
        for message in template {
            let rendered = self.handlebars.render_template(&message.message, &data)?;
            messages.push(Message {
                role: message.role.clone(),
                message: rendered,
            });
        }
        Ok(messages)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_prompt() {
        let prompt_template =
            PromptTemplate::new().from_prompt("prompt", "What is the capital of {{country}}");
        let mut context = Context::new();
        context.set("country", "France");
        let prompt = prompt_template.render_context("prompt", &context).unwrap();
        assert_eq!(
            prompt,
            vec![Message::single("What is the capital of France")]
        );
    }

    #[test]
    fn test_chat() {
        let prompt_template = PromptTemplate::new().from_chat(
            "prompt",
            vec![
                (
                    "system",
                    "You are NOT a master at {{subject}}. You know nothing about it.",
                ),
                ("user", "What is your favorite aspect of {{subject}}?"),
                ("ai", "I don't know anything about {{subject}}."),
            ],
        );
        let mut context = Context::new();
        context.set("subject", "math");
        let prompt = prompt_template.render_context("prompt", &context).unwrap();
        assert_eq!(
            prompt,
            vec![
                Message::chat(
                    Role::System,
                    "You are NOT a master at math. You know nothing about it."
                ),
                Message::chat(Role::User, "What is your favorite aspect of math?"),
                Message::chat(Role::Ai, "I don't know anything about math."),
            ]
        );
    }

    #[test]
    fn test_context() {
        let prompt_template = PromptTemplate::new()
            .from_chat("prompt", vec![("system", "This is my data: {{data}}.")]);

        let mut context = Context::new();
        context.set("data", serde_json::json!({"name": "gpt"}));
        let prompt = prompt_template.render_context("prompt", &context).unwrap();
        assert_eq!(
            prompt,
            vec![Message::chat(
                Role::System,
                "This is my data: {\"name\":\"gpt\"}."
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

        let prompt_template = PromptTemplate::new().from_chat(
            "prompt",
            vec![(
                "ai",
                "My name is {{name}} and I am {{#if (eq age 1)}}1 year{{else}}{{age}} years{{/if}} old.",
            )],
        );

        let data = Data {
            name: "gpt".to_string(),
            age: 5,
        };

        let prompt = prompt_template.render_data("prompt", data).unwrap();
        assert_eq!(
            prompt,
            vec![Message::chat(
                Role::Ai,
                "My name is gpt and I am 5 years old."
            )]
        );
    }
}
