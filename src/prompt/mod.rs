pub mod context;
use serde;
use serde::Serialize;

use anyhow::Result;
use chat::{Message, Role};
use handlebars::Handlebars;

use self::chat::{ASSISTANT_HELPER, SYSTEM_HELPER, USER_HELPER};

pub mod chat;

pub struct PromptEngine<'p> {
    /// A vector of template strings
    template: String,

    /// The handlebars template engine
    handlebars: Handlebars<'p>,
}

impl<'p> PromptEngine<'p> {
    /// Initialize a prompt template
    /// # Example
    /// ```rust
    /// use orca::prompt::prompt::PromptEngine;
    ///
    /// let prompt_template = PromptEngine::new("What is the capital of {{country}}");
    /// ```
    pub fn new(prompt: &str) -> PromptEngine<'p> {
        let mut handlebars = Handlebars::new();
        let template = prompt.to_string();
        handlebars.register_escape_fn(handlebars::no_escape);

        handlebars.register_helper("system", Box::new(SYSTEM_HELPER.clone()));
        handlebars.register_helper("user", Box::new(USER_HELPER.clone()));
        handlebars.register_helper("assistant", Box::new(ASSISTANT_HELPER.clone()));

        PromptEngine { template, handlebars }
    }

    /// Add a new template string to the prompt template
    /// # Example
    /// ```rust
    /// use orca::prompt::prompt::PromptEngine;
    /// use orca::prompt::Message;
    ///
    /// let mut prompt_template = PromptEngine::new("What is the capital of {{country}}");
    /// prompt_template.add_prompt("The capital is {{capital}}");
    /// ```
    pub fn add_to_prompt(&mut self, template: &str) {
        self.template.push_str(template);
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
    pub fn render<T>(&self, data: &T) -> Result<String>
    where
        T: Serialize,
    {
        let rendered = self.handlebars.render_template(&self.template, &data)?;
        Ok(rendered)
    }

    pub fn render_chat<T>(&self, data: &T) -> Result<Vec<Message>>
    where
        T: Serialize,
    {
        // let mut renderer = CaptureRenderer::new();
        // self.handlebars.render_template_to_write(&self.template, data, &mut renderer)?;
        Ok(Vec::new())
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

// #[cfg(test)]
// mod test {

//     use super::*;

//     #[test]
//     fn test_prompt() {
//         let prompt_template = prompt!("What is the capital of {{country}}");
//         let mut context = Context::new();
//         context.set("country", "France");
//         let prompt = prompt_template.render_context(&context).unwrap();
//         assert_eq!(prompt, vec![Message::single("What is the capital of France")]);
//     }

//     #[test]
//     fn test_chat() {
//         let prompt_template = prompts!(
//             (
//                 "system",
//                 "You are NOT a master at {{subject}}. You know nothing about it."
//             ),
//             ("user", "What is your favorite aspect of {{subject}}?"),
//             ("ai", "I don't know anything about {{subject}}.")
//         );
//         let mut context = Context::new();
//         context.set("subject", "math");
//         let prompt = prompt_template.render_context(&context).unwrap();
//         assert_eq!(
//             prompt,
//             vec![
//                 Message::chat(Role::System, "You are NOT a master at math. You know nothing about it."),
//                 Message::chat(Role::User, "What is your favorite aspect of math?"),
//                 Message::chat(Role::Ai, "I don't know anything about math."),
//             ]
//         );
//     }

//     #[test]
//     fn test_context() {
//         let prompt_template = prompts!(("system", "This is my data: {{data}}."));

//         let mut context = Context::new();
//         context.set(
//             "data",
//             serde_json::json!({"name": "gpt", "age": 5, "country": "France"}),
//         );
//         let prompt = prompt_template.render_context(&context).unwrap();
//         assert_eq!(
//             prompt,
//             vec![Message::chat(
//                 Role::System,
//                 "This is my data: {age:5,country:France,name:gpt}."
//             )]
//         );
//     }

//     #[test]
//     fn test_data() {
//         #[derive(Serialize)]
//         struct Data {
//             name: String,
//             age: u8,
//         }

//         let prompt_template = prompts!((
//             "ai",
//             "My name is {{name}} and I am {{#if (eq age 1)}}1 year{{else}}{{age}} years{{/if}} old.",
//         ));

//         let data = Data {
//             name: "gpt".to_string(),
//             age: 5,
//         };

//         let prompt = prompt_template.render(&data).unwrap();
//         assert_eq!(
//             prompt,
//             vec![Message::chat(Role::Ai, "My name is gpt and I am 5 years old.")]
//         );
//     }
// }
