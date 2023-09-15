use crate::llm::llm::{GenerateWithData, LLM};
use crate::llm::{error::LLMError, llm::GenerateWithContext};
use crate::prompt::context::Context;
use crate::prompt::prompt::PromptTemplate;
use serde::Serialize;

// /// LLM chain that formats a prompt and calls an LLM.
// ///
// /// # Example
// /// ```rust
// /// use orca::chains::llm::LLMChain;
// /// use orca::prompt
// /// use orca::llm::openai::client::OpenAIClient;
// ///
// /// let llm_chain = LLMChain::new().with_llm(OpenAIClient::new())
// ///                                .with_prompt(PromptTemplate::new().from_prompt("prompt", "What is the capital of {{country}}"));
// /// let res = llm_chain.generate("prompt", "France");
// /// ```
pub struct LLMChain<'a, T>
where
    T: Serialize,
{
    llm: Box<dyn LLM<T> + 'a>,
}

impl<'a, T> LLMChain<'a, T>
where
    T: Serialize,
{
    pub fn new(llm: impl LLM<T> + 'a) -> Self {
        LLMChain { llm: Box::new(llm) }
    }
}

impl<'a, T> LLM<T> for LLMChain<'a, T> where T: Serialize {}

#[async_trait::async_trait(?Send)]
impl<T> GenerateWithContext<T> for LLMChain<'_, T>
where
    T: Serialize,
{
    async fn generate_with_context<'a>(&'a self, name: &str, context: &Context<T>, template: &PromptTemplate) -> Result<String, LLMError> {
        self.llm.generate_with_context(name, context, template).await
    }
}

#[async_trait::async_trait(?Send)]
impl<T> GenerateWithData<T> for LLMChain<'_, T>
where
    T: Serialize,
{
    async fn generate_with_data<'a>(&'a self, name: &str, data: &T, template: &PromptTemplate) -> Result<String, LLMError> {
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
