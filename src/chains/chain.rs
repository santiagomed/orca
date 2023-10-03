use super::Chain;
use super::ChainResult;
use crate::llm::LLM;
use crate::memory::Memory;
use crate::prompt::PromptEngine;

use anyhow::Result;
use std::collections::HashMap;

/// Simple LLM chain that formats a prompt and calls an LLM.
///
/// # Example
/// ```rust
/// use orca::chains::chain::LLMChain;
/// use orca::chains::Chain;
/// use orca::prompts;
/// use orca::prompt::prompt::PromptEngine;
/// use orca::llm::openai::OpenAIClient;
/// use serde::Serialize;
/// use tokio;
///
/// #[derive(Serialize)]
/// pub struct Data {
///     country1: String,
///     country2: String,
/// }
///
/// #[tokio::main]
/// async fn main() {
///     let client = OpenAIClient::new();
///
///     let mut chain = LLMChain::new(&client).with_prompt(prompts!(
///         ("user", "What is the capital of {{country1}}"),
///         ("ai", "Paris"),
///         ("user", "What is the capital of {{country2}}")
///     ));
///     chain.load_context(&Data {
///         country1: "France".to_string(),
///         country2: "Germany".to_string(),
///     });
///     let res = chain.execute().await.unwrap();
///     assert!(res.content().to_lowercase().contains("berlin"));
/// }
/// ```
pub struct LLMChain<'llm> {
    /// The name of the LLMChain.
    pub name: String,

    /// The prompt template instance used by the LLMChain.
    pub prompt: PromptEngine<'llm>,

    /// The LLM used by the LLMChain.
    llm: &'llm (dyn LLM),

    /// Memory of the LLMChain.
    memory: Option<Box<dyn Memory>>,

    context: HashMap<String, String>,
}

impl<'llm> LLMChain<'llm> {
    /// Initialize a new LLMChain with an LLM. The LLM must implement the LLM trait.
    pub fn new(llm: &'llm impl LLM, prompt: &str) -> LLMChain<'llm> {
        LLMChain {
            name: uuid::Uuid::new_v4().to_string(),
            llm,
            prompt: PromptEngine::new(prompt),
            memory: None,
            context: HashMap::new(),
        }
    }

    /// Change the prompt template used by the LLMChain.
    pub fn with_prompt(mut self, prompt: PromptEngine<'llm>) -> Self {
        self.prompt = prompt;
        self
    }

    /// Change the memory used by the LLMChain.
    pub fn with_memory(mut self, memory: impl Memory + 'static) -> Self {
        self.memory = Some(Box::new(memory));
        self
    }
}

#[async_trait::async_trait(?Send)]
impl<'llm> Chain for LLMChain<'llm> {
    async fn execute(&mut self) -> Result<ChainResult> {
        let prompt = self.prompt.render(&self.context)?;
        let response = if let Some(memory) = &mut self.memory {
            let mem = memory.memory();
            mem.save(&prompt)?;
            self.llm.generate(&mem.to_string()?).await?
        } else {
            // Fix this to deal with string and chat messages
            self.llm.generate(&prompt).await?
        };
        Ok(ChainResult::new(self.name.clone()).with_llm_response(response))
    }

    fn context(&mut self) -> &mut HashMap<String, String> {
        &mut self.context
    }
}

impl<'llm> Clone for LLMChain<'llm> {
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
        llm::openai::OpenAIClient,
        memory, prompt,
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
        let client = OpenAIClient::new();
        let prompt = r#"
            {{#user}}
            What is the capital of {{country1}}?
            {{/user}}
            {{#assistant}}
            Paris
            {{/assistant}}
            {{#user}}
            What is the capital of {{country2}}?
            {{/user}}
            "#;
        let mut chain = LLMChain::new(&client, prompt);
        chain.load_context(&DataOne {
            country1: "France".to_string(),
            country2: "Germany".to_string(),
        });
        let res = chain.execute().await.unwrap().content();

        assert!(res.contains("Berlin") || res.contains("berlin"));
    }

    #[tokio::test]
    async fn test_generate_with_record() {
        let client = OpenAIClient::new().with_model("gpt-3.5-turbo-16k");
        let record = record::html::HTML::from_url("https://www.orwellfoundation.com/the-orwell-foundation/orwell/essays-and-other-works/shooting-an-elephant/")
            .await
            .unwrap()
            .with_selectors("p")
            .spin()
            .unwrap();

        let prompt = r#"
            {{#user}}
            Give a long summary of the following story: {{story}}
            {{/user}}
            "#;

        let mut chain = LLMChain::new(&client, prompt);

        chain.load_record("story", record);
        let res = chain.execute().await.unwrap().content();
        assert!(res.contains("elephant") || res.contains("burma"));
    }

    #[tokio::test]
    async fn test_generate_with_memory() {
        let client = OpenAIClient::new();

        let prompt = "{{#user}}My name is Orca{{/user}}";
        let mut chain = LLMChain::new(&client, prompt).with_memory(memory::ChatBuffer::new());
        chain.execute().await.unwrap();
        let mut chain = chain.with_prompt(prompt!("{{#user}}What is my name?{{/user}}"));
        let res = chain.execute().await.unwrap().content();

        assert!(res.to_lowercase().contains("orca"));
    }
}
