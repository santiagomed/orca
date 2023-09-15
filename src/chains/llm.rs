use crate::llm::llm::{GenerateWithData, LLM};
use crate::llm::{error::LLMError, llm::GenerateWithContext};
use crate::prompt::context::Context;
use crate::prompt::prompt::PromptTemplate;
use serde::Serialize;

/// LLM chain that formats a prompt and calls an LLM.
///
/// # Example
/// ```rust
/// use orca::chains::llm::LLMChain;
/// use orca::prompt;
/// use orca::prompt::prompt::PromptTemplate;
/// use orca::llm::openai::client::OpenAIClient;
/// use orca::llm::llm::GenerateWithData;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// pub struct Data {
///     country1: String,
///     country2: String,
/// }
///
/// async fn chain() {
///     let llm_chain = LLMChain::<Data>::new(OpenAIClient::new());
///     let prompt = prompt!(
///         "capital",
///         ("user", "What is the capital of {{country1}}"),
///         ("ai", "Paris"),
///         ("user", "What is the capital of {{country2}}")
///     );
///     let data = Data {
///         country1: "France".to_string(),
///         country2: "Germany".to_string(),
///     };
///     let response = llm_chain.generate_with_data("capital", &data, &prompt).await.unwrap();
/// }
/// ```
pub struct LLMChain<'llm, T>
where
    T: Serialize,
{
    llm: Box<dyn LLM<T> + 'llm>,
}

impl<'llm, T> LLMChain<'llm, T>
where
    T: Serialize,
{
    /// Initialize a new LLMChain with an LLM. The LLM must implement the LLM trait.
    pub fn new(llm: impl LLM<T> + 'llm) -> Self {
        LLMChain { llm: Box::new(llm) }
    }
}

impl<'llm, T> LLM<T> for LLMChain<'llm, T> where T: Serialize {}

#[async_trait::async_trait(?Send)]
impl<T> GenerateWithContext<T> for LLMChain<'_, T>
where
    T: Serialize,
{
    async fn generate_with_context<'llm>(&'llm self, name: &str, context: &Context<T>, template: &PromptTemplate) -> Result<String, LLMError> {
        self.llm.generate_with_context(name, context, template).await
    }
}

#[async_trait::async_trait(?Send)]
impl<T> GenerateWithData<T> for LLMChain<'_, T>
where
    T: Serialize,
{
    async fn generate_with_data<'llm>(&'llm self, name: &str, data: &T, template: &PromptTemplate) -> Result<String, LLMError> {
        self.llm.generate_with_data(name, data, template).await
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::{llm::openai::client::OpenAIClient, prompt};

    #[derive(Serialize)]
    pub struct Data {
        country1: String,
        country2: String,
    }

    #[tokio::test]
    async fn test_generate() {
        let client = OpenAIClient::new();

        let chain = LLMChain::<Data>::new(client);
        let prompt = prompt!(
            "capital",
            ("user", "What is the capital of {{country1}}"),
            ("ai", "Paris"),
            ("user", "What is the capital of {{country2}}")
        );

        let data = Data {
            country1: "France".to_string(),
            country2: "Germany".to_string(),
        };

        let response = chain.generate_with_data("capital", &data, &prompt).await.unwrap();
        assert!(response.to_lowercase().contains("berlin"));
    }
}
