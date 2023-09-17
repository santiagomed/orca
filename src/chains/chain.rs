use serde::Serialize;

use crate::llm::error::LLMError;
use crate::llm::llm::Generate;
use crate::prompt::prompt::PromptTemplate;

/// Simple LLM chain that formats a prompt and calls an LLM.
///
/// # Example
/// ```rust
/// use orca::chains::chain::LLMChain;
/// use orca::chains::chain::Execute;
/// use orca::prompt;
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
///         client,
///         prompt!(
///             "capital",
///             ("user", "What is the capital of {{country1}}"),
///             ("ai", "Paris"),
///             ("user", "What is the capital of {{country2}}")
///         ),
///     )
///     .execute(
///         "capital",
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
    llm: Box<dyn Generate + 'llm>,
    prompt: PromptTemplate<'llm>,
}

impl<'llm> LLMChain<'llm> {
    /// Initialize a new LLMChain with an LLM. The LLM must implement the LLM trait.
    pub fn new(llm: impl Generate + 'llm, prompt: PromptTemplate<'llm>) -> LLMChain<'llm> {
        LLMChain { llm: Box::new(llm), prompt }
    }

    /// Change the LLM used by the LLMChain.
    pub fn with_llm(mut self, llm: impl Generate + 'llm) -> Self {
        self.llm = Box::new(llm);
        self
    }

    /// Change the prompt template used by the LLMChain.
    pub fn with_prompt(mut self, prompt: PromptTemplate<'llm>) -> Self {
        self.prompt = prompt;
        self
    }
}

#[async_trait::async_trait(?Send)]
pub trait Execute<T> {
    /// Execute an LLM chain using a context and a prompt template.
    async fn execute(&mut self, name: &str, data: &T) -> Result<String, LLMError>;
}

#[async_trait::async_trait(?Send)]
impl<'llm, T> Execute<T> for LLMChain<'llm>
where
    T: Serialize,
{
    async fn execute(&mut self, name: &str, data: &T) -> Result<String, LLMError> {
        let prompt = self.prompt.render_data(name, data)?;
        self.llm.generate(&prompt).await
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::{llm::openai::client::OpenAIClient, prompt};
    use serde::Serialize;

    #[derive(Serialize)]
    pub struct Data {
        country1: String,
        country2: String,
    }

    #[tokio::test]
    async fn test_generate() {
        let client = OpenAIClient::new();

        let res = LLMChain::new(
            client,
            prompt!(
                "capital",
                ("user", "What is the capital of {{country1}}"),
                ("ai", "Paris"),
                ("user", "What is the capital of {{country2}}")
            ),
        )
        .execute(
            "capital",
            &Data {
                country1: "France".to_string(),
                country2: "Germany".to_string(),
            },
        )
        .await
        .unwrap();

        assert!(res.contains("Berlin") || res.contains("berlin"));
    }
}
