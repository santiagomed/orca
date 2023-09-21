use std::collections::HashMap;

use super::Chain;
use crate::llm::error::LLMError;
use crate::llm::Generate;
use crate::memory::memory;
use crate::memory::Memory;
use crate::prompt::prompt::PromptTemplate;
use crate::prompt::{Message, Role};

/// Simple LLM chain that formats a prompt and calls an LLM.
///
/// # Example
/// ```rust
/// use orca::chains::chain::LLMChain;
/// use orca::chains::Chain;
/// use orca::prompts;
/// use orca::prompt::prompt::PromptTemplate;
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
///     chain.set_context(&Data {
///         country1: "France".to_string(),
///         country2: "Germany".to_string(),
///     });
///     let res = chain.execute().await.unwrap();
///     assert!(res.contains("Berlin") || res.contains("berlin"));
/// }
/// ```
pub struct LLMChain<'llm> {
    /// The name of the LLMChain.
    name: String,

    /// The LLM used by the LLMChain.
    llm: &'llm (dyn Generate),

    /// The prompt template instance used by the LLMChain.
    prompt: PromptTemplate<'llm>,

    /// Memory of the LLMChain.
    memory: Box<dyn Memory<'llm> + 'llm>,

    context: HashMap<String, String>,
}

impl<'llm> LLMChain<'llm> {
    /// Initialize a new LLMChain with an LLM. The LLM must implement the LLM trait.
    pub fn new(llm: &'llm impl Generate) -> LLMChain<'llm> {
        LLMChain {
            name: uuid::Uuid::new_v4().to_string(),
            llm,
            prompt: PromptTemplate::new(),
            memory: Box::new(memory::Buffer::new()),
            context: HashMap::new(),
        }
    }

    /// Change the prompt template used by the LLMChain.
    pub fn with_prompt(mut self, prompt: PromptTemplate<'llm>) -> Self {
        self.prompt = prompt;
        self
    }

    /// Change the memory used by the LLMChain.
    pub fn with_memory(mut self, memory: impl Memory<'llm> + 'llm) -> Self {
        self.memory = Box::new(memory);
        self
    }

    /// Get the name of the LLMChain.
    pub fn get_name(&self) -> &String {
        &self.name
    }

    /// Get the prompt template used by the LLMChain.
    pub fn get_prompt(&mut self) -> &mut PromptTemplate<'llm> {
        &mut self.prompt
    }
}

#[async_trait::async_trait(?Send)]
impl<'llm> Chain for LLMChain<'llm> {
    async fn execute(&mut self) -> Result<String, LLMError> {
        let msgs = self.prompt.render(&self.context)?;
        let prompt = self.memory.get_memory();
        prompt.extend(msgs);
        let response = self.llm.generate(&prompt).await?;
        prompt.push(Message::chat(Role::Ai, &response));
        Ok(response)
    }

    fn get_context(&mut self) -> &mut HashMap<String, String> {
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

    use std::vec;

    use super::*;
    use crate::{
        llm::openai::OpenAIClient,
        prompt, prompts,
        record::{self, spin::Spin},
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

        let mut chain = LLMChain::new(&client).with_prompt(prompts!(
            ("user", "What is the capital of {{country1}}"),
            ("ai", "Paris"),
            ("user", "What is the capital of {{country2}}")
        ));
        chain.set_context(&DataOne {
            country1: "France".to_string(),
            country2: "Germany".to_string(),
        });
        let res = chain.execute().await.unwrap();

        assert!(res.contains("Berlin") || res.contains("berlin"));
    }

    #[tokio::test]
    async fn test_generate_with_record() {
        let client = OpenAIClient::new().with_model("gpt-3.5-turbo-16k");
        let record = record::html::HTML::from_url("https://www.orwellfoundation.com/the-orwell-foundation/orwell/essays-and-other-works/shooting-an-elephant/", "p, li")
            .await
            .unwrap()
            .spin()
            .unwrap();

        let mut chain = LLMChain::new(&client).with_prompt(prompts!((
            "system",
            "Give a long summary of the following story:\n{{story}}"
        )));

        chain.set_record("story", record);
        let res = chain.execute().await.unwrap().to_lowercase();
        assert!(res.contains("elephant") || res.contains("burma"));
    }

    #[tokio::test]
    async fn test_generate_with_memory() {
        let client = OpenAIClient::new();

        let mut chain = LLMChain::new(&client).with_prompt(prompts!(("user", "My name is Orca")));
        chain.execute().await.unwrap();
        let mut chain = chain.with_prompt(prompt!("What is my name?"));
        let res = chain.execute().await.unwrap();

        assert!(res.contains("Orca"));
        assert_eq!(chain.memory.get_memory().len(), 4);
    }
}
