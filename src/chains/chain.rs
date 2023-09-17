use serde::Serialize;
use std::collections::HashMap;

use crate::llm::error::LLMError;
use crate::llm::llm::Generate;
use crate::prompt::prompt::PromptTemplate;
use crate::record::record::Record;

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
    /// The LLM used by the LLMChain.
    llm: Box<dyn Generate + 'llm>,

    /// The prompt template instance used by the LLMChain.
    prompt: PromptTemplate<'llm>,

    /// Any record used by the LLMChain.
    records: HashMap<String, Record>,
}

impl<'llm> LLMChain<'llm> {
    /// Initialize a new LLMChain with an LLM. The LLM must implement the LLM trait.
    pub fn new(llm: impl Generate + 'llm, prompt: PromptTemplate<'llm>) -> LLMChain<'llm> {
        LLMChain {
            llm: Box::new(llm),
            prompt,
            records: HashMap::new(),
        }
    }

    /// Change the LLM used by the LLMChain.
    pub fn with_llm(mut self, llm: impl Generate + 'llm) -> Self {
        self.llm = Box::new(llm);
        self
    }

    /// Specify a record to be used by the LLMChain.
    pub fn with_record(mut self, name: &str, record: Record) -> Self {
        self.records.insert(name.to_string(), record);
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
pub trait ExecuteWithRecord {
    /// Execute an LLM chain using a record and a prompt template.
    ///
    /// [TODO]:
    /// Creating a new hashmap when we already have one with records is not clean code
    /// since we created the record object to standardize the way we handle records.
    async fn execute_with_record(&mut self, prompt_name: &str, record_name: &str) -> Result<String, LLMError>;
}

#[async_trait::async_trait(?Send)]
impl<'llm> ExecuteWithRecord for LLMChain<'llm> {
    async fn execute_with_record(&mut self, prompt_name: &str, record_name: &str) -> Result<String, LLMError> {
        let mut h = HashMap::<String, String>::new();
        h.insert(record_name.to_string(), self.records[record_name].content.to_string());
        let prompt = self.prompt.render_data(prompt_name, &h)?;
        self.llm.generate(&prompt).await
    }
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
    use crate::{
        llm::openai::client::OpenAIClient,
        prompt,
        record::{self, spin::Spin},
    };
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

    #[tokio::test]
    async fn test_generate_with_record() {
        let client = OpenAIClient::new().with_model("gpt-3.5-turbo-16k");
        let record = record::html::HTML::from_url("https://www.orwellfoundation.com/the-orwell-foundation/orwell/essays-and-other-works/shooting-an-elephant/", "p, li")
            .await
            .unwrap()
            .spin()
            .unwrap();
        let res = LLMChain::new(
            client,
            prompt!(
                "story_summary",
                ("system", "Give a long summary of the following story:\n{{story}}")
            ),
        )
        .with_record("story", record)
        .execute_with_record("story_summary", "story")
        .await
        .unwrap();
        assert!(res.contains("elephant"));
    }
}
