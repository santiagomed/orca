use std::sync::Arc;

use serde::Serialize;

use super::Execute;
use crate::llm::error::LLMError;
use crate::llm::Generate;
use crate::memory::Memory;
use crate::prompt::prompt::PromptTemplate;

/// Simple LLM chain that formats a prompt and calls an LLM.
///
/// # Example
/// ```rust
/// use orca::chains::chain::LLMChain;
/// use orca::chains::traits::Execute;
/// use orca::prompts;
/// use orca::prompt::prompt::PromptTemplate;
/// use orca::llm::openai::client::OpenAIClient;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// pub struct Data {
///     country1: String,
///     country2: String,
/// }
///
/// async fn test_generate() {
///     let client = OpenAIClient::new();
///     let res = LLMChain::new(
///         &client,
///         prompts!(
///             ("user", "What is the capital of {{country1}}"),
///             ("ai", "Paris"),
///             ("user", "What is the capital of {{country2}}")
///         ),
///     )
///     .execute(
///         &Data {
///             country1: "France".to_string(),
///             country2: "Germany".to_string(),
///         },
///     )
///     .await
///     .unwrap();
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
    memory: (dyn Memory)
}

impl<'llm> LLMChain<'llm> {
    /// Initialize a new LLMChain with an LLM. The LLM must implement the LLM trait.
    pub fn new(llm: &'llm impl Generate, prompt: PromptTemplate<'llm>) -> LLMChain<'llm> {
        LLMChain {
            name: uuid::Uuid::new_v4().to_string(),
            llm,
            prompt,
            memory: None,
        }
    }

    /// Change the LLM used by the LLMChain.
    pub fn with_llm(mut self, llm: &'llm impl Generate) -> Self {
        self.llm = llm;
        self
    }

    /// Change the prompt template used by the LLMChain.
    pub fn with_prompt(mut self, prompt: PromptTemplate<'llm>) -> Self {
        self.prompt = prompt;
        self
    }

    /// Change the memory used by the LLMChain.
    pub fn with_memory(mut self, memory: impl Memory + 'llm) -> Self {
        self.memory = memory;
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
impl<'llm, T> Execute<T> for LLMChain<'llm>
where
    T: Serialize,
{
    async fn execute(&mut self, data: &T) -> Result<String, LLMError> {
        let prompt = self.prompt.render_data(data)?;
        let response = self.llm.generate(&prompt).await?;
        self.memory.load_memory(prompt);
        Ok(response)
    }
}

impl<'llm> Clone for LLMChain<'llm> {
    fn clone(&self) -> Self {
        LLMChain {
            name: self.name.clone(),
            llm: self.llm.clone(),
            prompt: self.prompt.clone(),
            memory: self.memory.clone(),
        }
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::{
        llm::openai::OpenAIClient,
        prompts,
        record::{self, spin::Spin}, prompt::prompt::Message,
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

        let res = LLMChain::new(
            &client,
            prompts!(
                ("user", "What is the capital of {{country1}}"),
                ("ai", "Paris"),
                ("user", "What is the capital of {{country2}}")
            ),
        )
        .execute(&DataOne {
            country1: "France".to_string(),
            country2: "Germany".to_string(),
        })
        .await
        .unwrap();

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

        let res = LLMChain::new(
            &client,
            prompts!(("system", "Give a long summary of the following story:\n{{story}}")),
        )
        .execute(&DataTwo {
            story: record.content.to_string(),
        })
        .await
        .unwrap();
        assert!(res.contains("elephant"));
    }

    #[tokio::test]
    async fn test_generate_with_memory() {
        let client = OpenAIClient::new();
        let mut memory = Buffer::new();

    }
}
