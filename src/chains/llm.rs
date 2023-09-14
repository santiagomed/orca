use serde::Serialize;

use crate::llm::error::LLMError;
use crate::llm::llm::LLM;
use crate::prompt::context::Context;
use crate::prompt::prompt::PromptTemplate;

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
pub struct LLMChain<'a, T, L>
where
    T: Serialize + std::marker::Sync + std::fmt::Display,
    L: LLM<T>,
{
    llm: Box<L>,
    prompt_template: PromptTemplate<'a>,
    _t: std::marker::PhantomData<T>,
}

impl<'a, T, L> LLMChain<'a, T, L>
where
    T: Serialize + std::marker::Sync + std::fmt::Display,
    L: LLM<T>,
{
    pub fn new(llm: L) -> Self {
        LLMChain {
            llm: Box::new(llm),
            prompt_template: PromptTemplate::new(),
            _t: std::marker::PhantomData,
        }
    }

    pub fn with_prompt(mut self, prompt_template: PromptTemplate<'a>) -> Self {
        self.prompt_template = prompt_template;
        self
    }
}

#[async_trait::async_trait]
pub trait Execute<T, L> {
    async fn execute_context(&self, name: &str, data: &Context<T>) -> Result<String, LLMError>;

    async fn execute_data(&self, name: &str, data: &T) -> Result<String, LLMError>;
}

#[async_trait::async_trait]
impl<'a, T, L> Execute<T, L> for LLMChain<'a, T, L>
where
    T: Serialize + std::marker::Sync + std::fmt::Display + std::marker::Send,
    L: LLM<T> + std::marker::Send + std::marker::Sync,
{
    async fn execute_context(&self, name: &str, context: &Context<T>) -> Result<String, LLMError> {
        Ok(self.llm.generate_with_context(name, context, &self.prompt_template).await?)
    }

    async fn execute_data(&self, name: &str, data: &T) -> Result<String, LLMError> {
        Ok(self.llm.generate_with_data(name, data, &self.prompt_template).await?)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        llm::openai::client::{OpenAIClient, OpenAIConfig},
        prompt,
    };

    #[tokio::test]
    async fn test_generate() {
        let client = OpenAIClient::new();
        let chain = LLMChain::<&str, OpenAIClient<OpenAIConfig>>::new(client).with_prompt(prompt!(
            "capital",
            ("user", "What is the capital of {{country}}"),
            ("ai", "Paris"),
            ("user", "What is the capital of {{country2}}")
        ));

        let mut context = Context::new();
        context.set("country1", "France");
        context.set("country2", "Germany");

        let response = chain.execute_context("capital", &context).await.unwrap();
        assert!(response.to_lowercase().contains("berlin"));
    }
}
