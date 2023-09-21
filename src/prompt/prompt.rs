use super::{context::Context, error::PromptTemplateError, Message};
use handlebars::Handlebars;
use serde::Serialize;

pub struct PromptTemplate<'p> {
    /// A vector of template strings
    template: Vec<Message>,

    /// The handlebars template engine
    handlebars: Handlebars<'p>,
}

impl<'p> PromptTemplate<'p> {
    /// Initialize a prompt template
    /// # Example
    /// ```rust
    /// use orca::prompt::prompt::PromptTemplate;
    ///
    /// let mut prompt_template = PromptTemplate::new();
    /// ```
    pub fn new() -> PromptTemplate<'p> {
        let mut handlebars = Handlebars::new();
        let template = Vec::new();
        handlebars.register_escape_fn(handlebars::no_escape);

        PromptTemplate { template, handlebars }
    }

    /// Initialize a prompt template with a single template string
    /// If chat format is desired, use from_chat instead
    /// # Example
    /// ```rust
    /// use orca::prompt::prompt::PromptTemplate;
    ///
    /// let mut prompt_template = PromptTemplate::new().from_prompt("What is the capital of {{country}}");
    /// ```
    pub fn from_prompt(mut self, template: &str) -> PromptTemplate<'p> {
        self.template = vec![Message::single(template)];
        self
    }

    /// Initialize a prompt template with multiple template strings
    /// If single format is desired, use from_prompt instead
    /// # Example
    /// ```rust
    /// use orca::prompt::prompt::PromptTemplate;
    ///
    /// let mut prompt_template = PromptTemplate::new().from_chat(vec![
    ///    ("system", "You are NOT a master at {{subject}}. You know nothing about it."),
    ///    ("user", "What is your favorite aspect of {{subject}}?"),
    ///    ("ai", "I don't know anything about {{subject}}."),
    /// ]);
    /// ```
    pub fn from_chat(mut self, templates: Vec<(&str, &str)>) -> PromptTemplate<'p> {
        self.template = Message::into_vec(templates);
        self
    }

    /// Add a new template string to the prompt template
    /// # Example
    /// ```rust
    /// use orca::prompt::prompt::PromptTemplate;
    /// use orca::prompt::Message;
    ///
    /// let mut prompt_template = PromptTemplate::new().from_prompt("What is the capital of {{country}}");
    /// prompt_template.add_prompt(("ai", "The capital is {{capital}}"));
    /// ```
    pub fn add_prompt(&mut self, template: (&str, &str)) {
        self.template.push(Message::chat(template.0.into(), template.1));
    }

    /// Render a prompt template
    /// # Example
    /// ```rust
    /// use orca::prompt::{prompt::PromptTemplate, context::Context};
    /// use orca::prompt::{Message, Role};
    ///
    /// let mut prompt_template = PromptTemplate::new().from_prompt("Your name is {{name}}");
    /// let mut context = Context::new();
    /// context.set("name", "gpt");
    /// let prompt = prompt_template.render_context(&context).unwrap();
    /// assert_eq!(prompt, vec![Message::single("Your name is gpt")]);
    /// ```
    pub fn render_context<T>(&self, context: &Context<T>) -> Result<Vec<Message>, PromptTemplateError>
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
    /// use orca::prompt::{prompt::{PromptTemplate}, context::Context};
    /// use orca::prompt::{Message, Role};
    /// use serde::Serialize;
    ///
    /// #[derive(Serialize)]
    /// struct Data {
    ///    name: String,
    ///    age: u8,
    /// }
    ///
    /// let mut prompt_template = PromptTemplate::new().from_chat(vec![
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
    pub fn render<T>(&self, data: &T) -> Result<Vec<Message>, PromptTemplateError>
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

impl<'p> Clone for PromptTemplate<'p> {
    /// Clone a prompt template
    fn clone(&self) -> Self {
        PromptTemplate {
            template: self.template.clone(),
            handlebars: self.handlebars.clone(),
        }
    }
}

#[macro_export]
macro_rules! prompt {
    ($template:expr) => {
        PromptTemplate::new().from_prompt($template)
    };
}

#[macro_export]
macro_rules! prompts {
    ($($template:expr),+) => {
        PromptTemplate::new().from_chat(vec![$($template),+])
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
