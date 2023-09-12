use std::collections::HashMap;

use super::context::Context;
use handlebars::Handlebars;

#[derive(PartialEq, Debug)]
pub enum Prompt {
    /// A single prompt string
    /// # Example
    /// ```text
    /// "What is your name?"
    /// ```
    Single(String),

    /// A multiple prompt strings
    /// # Example
    /// ```text
    /// (System, "You are NOT a master at math. You know nothing about it.")
    /// (User, "What is your favorite aspect of math?")
    /// (Ai, "I don't know anything about math.")
    /// ```
    Chat(Vec<(Role, String)>),
}

#[derive(Debug, Clone)]
enum Template {
    /// A single template string
    /// # Example
    /// ```text
    /// "How do you learn {{subject}}?"
    /// ```
    Single(String),

    /// A multiple template strings
    /// # Example
    /// ```text
    /// (System, "You are NOT a master at {{subject}}.")
    /// (User, "What is your favorite aspect of {{subject}}?")
    /// (Ai, "I don't know anything about {{subject}}.")
    /// ```
    Chat(Vec<(Role, String)>),
}

impl From<Vec<(&str, &str)>> for Template {
    fn from(v: Vec<(&str, &str)>) -> Template {
        let mut templates = Vec::new();
        for (role, template) in v {
            let role = Role::from(role);
            templates.push((role, template.to_string()));
        }
        Template::Chat(templates)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Role {
    /// The system role
    System,

    /// The user role
    User,

    /// The AI role
    Ai,
}

/// Trait for converting a role to a string
impl ToString for Role {
    fn to_string(&self) -> String {
        match self {
            Role::System => "system".to_string(),
            Role::User => "user".to_string(),
            Role::Ai => "ai".to_string(),
        }
    }
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
    templates: HashMap<String, Template>,

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
            .insert(name.to_string(), Template::Single(template.to_string()));
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
            .insert(name.to_string(), Template::from(templates));
        self
    }

    /// Render a prompt template
    /// # Example
    /// ```rust
    /// use orcha::prompt::{prompt::{Prompt, PromptTemplate}, context::Context};
    ///
    /// let mut prompt_template = PromptTemplate::new().from_prompt("prompt", "Your name is {{name}}");
    /// let mut context = Context::new();
    /// context.set("name", "gpt");
    /// let prompt = prompt_template.render("prompt", &context).unwrap();
    /// assert_eq!(prompt, Prompt::Single("Your name is gpt".to_string()));
    /// ```
    pub fn render(&self, name: &str, context: &Context) -> Result<Prompt, handlebars::RenderError> {
        let template = self.templates.get(name).unwrap();
        match template {
            Template::Single(template) => {
                let prompt = self
                    .handlebars
                    .render_template(template, context.variables())?;
                Ok(Prompt::Single(prompt))
            }
            Template::Chat(templates) => {
                let mut prompts = Vec::new();
                for (role, template) in templates {
                    let prompt = self
                        .handlebars
                        .render_template(template, context.variables())?;
                    prompts.push((role.clone(), prompt));
                }
                Ok(Prompt::Chat(prompts))
            }
        }
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
        let prompt = prompt_template.render("prompt", &context).unwrap();
        assert_eq!(
            prompt,
            Prompt::Single("What is the capital of France".to_string())
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
        let prompt = prompt_template.render("prompt", &context).unwrap();
        assert_eq!(
            prompt,
            Prompt::Chat(vec![
                (
                    Role::System,
                    "You are NOT a master at math. You know nothing about it.".to_string()
                ),
                (
                    Role::User,
                    "What is your favorite aspect of math?".to_string()
                ),
                (Role::Ai, "I don't know anything about math.".to_string()),
            ])
        );
    }
}
