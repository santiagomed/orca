use super::Chain;
use super::ChainResult;
use crate::llm::LLM;
use crate::memory::Memory;
use crate::prompt::TemplateEngine;

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Represents the simples chain for a Large Language Model (LLM).
///
/// This simple chain just takes a prompt/template and generates a response using the LLM.
/// It can make use of context, memory, and a prompt template.
pub struct LLMChain {
    /// The unique identifier for this LLMChain.
    pub name: String,

    /// The prompt template engine instance that is used by the LLMChain
    /// to generate the actual prompts based on the given context.
    pub prompt: TemplateEngine,

    /// A reference to the LLM that this chain will use to process the prompts.
    llm: Arc<dyn LLM>,

    /// Memory associated with the LLMChain. It can be used to persist
    /// state or data across different executions of the chain.
    memory: Option<Arc<Mutex<dyn Memory>>>,

    /// The context containing key-value pairs which the `prompt`
    /// template engine might use to render the final prompt.
    context: HashMap<String, String>,
}

impl LLMChain {
    /// Creates a new LLMChain given an LLM and a prompt template.
    ///
    /// # Examples
    /// ```rust
    /// use orca::llm::openai::OpenAI;
    /// use orca::llm::LLM;
    /// use orca::prompt::TemplateEngine;
    /// use orca::chains::chain::LLMChain;
    /// use std::sync::Arc;
    ///
    /// let client = Arc::new(OpenAI::new());
    /// let prompt = "Hello, LLM!";
    /// let chain = LLMChain::new(client.clone(), prompt);
    /// ```
    pub fn new(llm: Arc<dyn LLM>) -> LLMChain {
        LLMChain {
            name: uuid::Uuid::new_v4().to_string(),
            llm,
            prompt: TemplateEngine::new(),
            memory: None,
            context: HashMap::new(),
        }
    }

    /// Modifies the LLMChain's prompt template.
    ///
    /// This is a builder-style method that returns a mutable reference to `self`.
    ///
    /// # Examples
    /// ```rust
    /// use orca::llm::openai::OpenAI;
    /// use orca::llm::LLM;
    /// use orca::prompt::TemplateEngine;
    /// use orca::chains::chain::LLMChain;
    /// use orca::template;
    /// use std::sync::Arc;
    ///
    /// let client = Arc::new(OpenAI::new());
    /// let prompt = "Hello, LLM!";
    /// let mut chain = LLMChain::new(client.clone(), prompt);
    /// let new_prompt = "Hello, LLM! How are you?";
    /// let chain = chain.with_prompt(template!(new_prompt));
    /// ```
    pub fn with_prompt(self, name: &str, prompt: &str) -> Self {
        Self {
            prompt: self.prompt.register_template(name, prompt),
            ..self
        }
    }

    pub fn duplicate_template(&mut self, name: &str) -> Option<String> {
        let template_name = format!("{}-{}", name, uuid::Uuid::new_v4().to_string());
        if let Some(template) = self.prompt.get_template(name) {
            let mut prompt_clone = self.prompt.clone();
            prompt_clone = prompt_clone.register_template(template_name.as_str(), &template);
            self.prompt = prompt_clone;
        } else {
            return None;
        }
        Some(template_name)
    }

    /// Change the memory used by the LLMChain.
    ///
    /// This is a builder-style method that returns a mutable reference to `self`.
    ///
    /// # Examples
    /// ```rust
    /// use orca::llm::openai::OpenAI;
    /// use orca::llm::LLM;
    /// use orca::prompt::TemplateEngine;
    /// use orca::chains::chain::LLMChain;
    /// use orca::memory::ChatBuffer;
    /// use std::sync::Arc;
    ///
    /// let client = Arc::new(OpenAI::new());
    /// let prompt = "Hello, LLM!";
    /// let mut chain = LLMChain::new(client.clone(), prompt);
    /// let memory = ChatBuffer::new();
    /// let chain = chain.with_memory(memory);
    /// ```
    pub fn with_memory<T: Memory + 'static>(mut self, memory: T) -> Self {
        self.memory = Some(Arc::new(Mutex::new(memory)));
        self
    }
}

#[async_trait::async_trait]
impl Chain for LLMChain {
    async fn execute(&self, target: &str) -> Result<ChainResult> {
        let prompt = self.prompt.render_context(target, &self.context)?;

        let response = if let Some(memory) = &self.memory {
            let mut locked_memory = memory.lock().await; // Lock the memory
            let mem = locked_memory.memory();
            mem.save(prompt)?;
            self.llm.generate(mem.clone_prompt()).await?
        } else {
            self.llm.generate(prompt.clone_prompt()).await?
        };

        Ok(ChainResult::new(self.name.clone()).with_llm_response(response))
    }

    fn context(&mut self) -> &mut HashMap<String, String> {
        &mut self.context
    }
}

impl Clone for LLMChain {
    fn clone(&self) -> Self {
        LLMChain {
            name: self.name.clone(),
            llm: self.llm.clone(),
            prompt: self.prompt.clone(),
            memory: self.memory.clone(),
            context: self.context.clone(),
        }
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::{
        llm::openai::OpenAI,
        memory,
        record::{self, Spin},
    };
    use serde::Serialize;

    #[derive(Serialize)]
    pub struct DataOne {
        country1: String,
        country2: String,
    }

    #[derive(Serialize)]
    pub struct DataTwo {
        story: String,
    }

    #[tokio::test]
    async fn test_generate() {
        let client = Arc::new(OpenAI::new());
        let prompt = r#"
            {{#chat}}
            {{#user}}
            What is the capital of {{country1}}?
            {{/user}}
            {{#assistant}}
            Paris
            {{/assistant}}
            {{#user}}
            What is the capital of {{country2}}?
            {{/user}}
            {{/chat}}
            "#;
        let mut chain = LLMChain::new(client).with_prompt("capitals", prompt);
        chain
            .load_context(&DataOne {
                country1: "France".to_string(),
                country2: "Germany".to_string(),
            })
            .await;
        let res = chain.execute("capitals").await.unwrap().content();

        assert!(res.contains("Berlin") || res.contains("berlin"));
    }

    #[tokio::test]
    async fn test_generate_with_record() {
        let client = Arc::new(OpenAI::new().with_model("gpt-3.5-turbo-16k"));
        let record = record::html::HTML::from_url("https://www.orwellfoundation.com/the-orwell-foundation/orwell/essays-and-other-works/shooting-an-elephant/")
            .await
            .unwrap()
            .with_selectors("p")
            .spin()
            .unwrap();

        let prompt = r#"
            {{#chat}}
            {{#user}}
            Give a long summary of the following story: {{story}}
            {{/user}}
            {{/chat}}
            "#;

        let mut chain = LLMChain::new(client).with_prompt("summary", prompt);

        chain.load_record("story", record);
        let res = chain.execute("summary").await.unwrap().content();
        assert!(res.contains("elephant") || res.contains("burma"));
    }

    #[tokio::test]
    async fn test_generate_with_memory() {
        let client = Arc::new(OpenAI::new());

        let prompt = "{{#chat}}{{#user}}My name is Orca{{/user}}{{/chat}}";
        let chain = LLMChain::new(client).with_prompt("name", prompt).with_memory(memory::ChatBuffer::new());
        chain.execute("name").await.unwrap();
        let chain = chain.with_prompt("name", "{{#chat}}{{#user}}What is my name?{{/user}}{{/chat}}");
        let res = chain.execute("name").await.unwrap().content();

        assert!(res.to_lowercase().contains("orca"));
    }
}
