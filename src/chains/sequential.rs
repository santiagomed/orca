use std::collections::HashMap;

use crate::prompt::clean_prompt;

use super::chain::LLMChain;
use super::{Chain, ChainResult};
use anyhow::Result;

pub struct SequentialChain<'llm> {
    /// The name of the LLMChain.
    name: String,

    /// Vector of LLM chains used by the SequentialChain.
    chains: Vec<LLMChain<'llm>>,

    /// The context for for the templates used by the SequentialChain.
    context: HashMap<String, String>,
}

impl<'llm> Default for SequentialChain<'llm> {
    fn default() -> Self {
        Self {
            name: uuid::Uuid::new_v4().to_string(),
            chains: Vec::new(),
            context: HashMap::new(),
        }
    }
}

impl<'llm> SequentialChain<'llm> {
    /// Initialize a new sequential chain.
    pub fn new() -> SequentialChain<'llm> {
        SequentialChain::default()
    }

    /// Add a simple LLM Chain to the sequential chain.
    pub fn link(mut self, chain: LLMChain<'llm>) -> SequentialChain<'llm> {
        self.chains.push(chain);
        self
    }
}

pub fn format_prompt_as_user(prompt: &mut str) -> String {
    format!("{{{{#user}}}}{}{{{{/user}}}}", clean_prompt(prompt, true))
}

#[async_trait::async_trait(?Send)]
impl<'llm> Chain for SequentialChain<'llm> {
    fn execute(&mut self) -> Result<ChainResult> {
        let mut response = String::new();
        let mut result: ChainResult = ChainResult::new(self.name.to_string()); // initialize result to a default value
        for chain in &mut self.chains {
            if !response.is_empty() {
                chain.prompt.add_to_template(&format_prompt_as_user(&mut response));
            }
            result = chain.execute()?;
            response = result.content();
        }
        Ok(result)
    }

    fn context(&mut self) -> &mut HashMap<String, String> {
        &mut self.context
    }

    fn load_context<T>(&mut self, context: &T)
    where
        T: serde::Serialize,
    {
        for chain in &mut self.chains {
            chain.load_context(context);
        }
    }
}

#[cfg(test)]
mod test {

    use std::sync::Arc;

    use super::*;
    use crate::llm::openai::OpenAIClient;
    use serde::Serialize;

    #[derive(Serialize)]
    pub struct Data {
        play: String,
    }

    #[tokio::test]
    async fn test_generate() {
        let client = Arc::new(OpenAIClient::new());

        let first = "{{#chat}}{{#user}}Give me a summary of {{play}}'s plot.{{/user}}{{/chat}}";
        let second = "{{#chat}}{{#system}}You are a professional critic. When given a summary of a play, you must write a review of it. Here is a summary of {{play}}'s plot:{{/system}}{{/chat}}";

        let mut chain = SequentialChain::new()
            .link(LLMChain::new(client.clone(), first))
            .link(LLMChain::new(client, second));
        chain.load_context(&Data {
            play: "Hamlet".to_string(),
        });
        let res = chain.execute();
        assert!(res.is_ok());
    }
}
