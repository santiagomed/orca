use std::collections::HashMap;

use super::chain::LLMChain;
use super::Chain;
use crate::llm::error::LLMError;

pub struct SequentialChain<'llm> {
    chains: Vec<LLMChain<'llm>>,
    context: HashMap<String, String>,
}

impl<'llm> SequentialChain<'llm> {
    /// Initialize a new sequential chain.
    pub fn new() -> SequentialChain<'llm> {
        SequentialChain {
            chains: Vec::new(),
            context: HashMap::new(),
        }
    }

    /// Add a simple LLM Chain to the sequential chain.
    pub fn link(mut self, chain: LLMChain<'llm>) -> SequentialChain<'llm> {
        self.chains.push(chain);
        self
    }
}

#[async_trait::async_trait(?Send)]
impl<'llm> Chain for SequentialChain<'llm> {
    async fn execute(&mut self) -> Result<String, LLMError> {
        let mut response = String::new();
        for chain in &mut self.chains {
            if !response.is_empty() {
                let prompt = chain.get_prompt();
                prompt.add_prompt(("user", &response));
            }
            response = chain.execute().await?;
        }
        Ok(response)
    }

    fn get_context(&mut self) -> &mut HashMap<String, String> {
        &mut self.context
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::prompt::prompt::PromptTemplate;
    use crate::{llm::openai::OpenAIClient, prompt, prompts};
    use serde::Serialize;

    #[derive(Serialize)]
    pub struct Data {
        play: String,
    }

    #[tokio::test]
    async fn test_generate() {
        let client = OpenAIClient::new();

        let mut chain = SequentialChain::new()
            .link(LLMChain::new(&client).with_prompt(prompt!("Give me a summary of {{play}}'s plot.")))
            .link(LLMChain::new(&client).with_prompt(prompts!(("ai", "You are a professional critic. When given a summary of a play, you must write a review of it. Here is a summary of {{play}}'s plot:"))));
        chain.set_context(&Data {
            play: "Hamlet".to_string(),
        });
        let res = chain.execute().await;
        assert!(res.is_ok());
    }
}
