use super::chain::LLMChain;
use super::traits::Execute;
use crate::llm::error::LLMError;
use serde::Serialize;

pub struct SequentialChain<'llm> {
    chains: Vec<LLMChain<'llm>>,
}

impl<'llm> SequentialChain<'llm> {
    /// Initialize a new sequential chain.
    pub fn new() -> SequentialChain<'llm> {
        println!("< Initializing a new sequential chain. >");
        SequentialChain { chains: Vec::new() }
    }

    /// Add a simple LLM Chain to the sequential chain.
    pub fn link(mut self, chain: LLMChain<'llm>) -> SequentialChain<'llm> {
        println!("< Adding a {:#?} to the sequential chain. >", chain.get_name());
        self.chains.push(chain);
        self
    }
}

#[async_trait::async_trait(?Send)]
impl<'llm, T> Execute<T> for SequentialChain<'llm>
where
    T: Serialize,
{
    async fn execute(&mut self, data: &T) -> Result<String, LLMError> {
        println!("< Executing a sequential chain. >");
        let mut response = String::new();
        for chain in &mut self.chains {
            if !response.is_empty() {
                let prompt = chain.get_prompt();
                prompt.add_prompt(("user", &response));
            }
            response = chain.execute(data).await?;
        }
        Ok(response)
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::prompt::prompt::PromptTemplate;
    use crate::{chain, llm::openai::client::OpenAIClient, prompt, prompts};
    use serde::Serialize;

    #[derive(Serialize)]
    pub struct Data {
        play: String,
    }

    #[tokio::test]
    async fn test_generate() {
        let client = OpenAIClient::new();

        let res = SequentialChain::new()
            .link(chain!("Summary", &client, prompt!("Give me a summary of {{play}}'s plot.")))
            .link(chain!("Critic", &client, prompts!(("ai", "You are a professional critic. When given a summary of a play, you must write a review of it. Here is a summary of {{play}}'s plot:"))))
            .execute(&Data {
                play: "Hamlet".to_string(),
            })
            .await;
        assert!(res.is_ok());
    }
}
