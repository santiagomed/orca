use serde::Serialize;
use std::collections::HashMap;

use crate::chains::traits::{Execute, ExecuteWithRecord};
use crate::llm::error::LLMError;
use crate::llm::llm::Generate;
use crate::prompt::prompt::PromptTemplate;
use crate::record::record::Record;

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
///         Some("MyChain"),
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

    /// Any record used by the LLMChain.
    records: HashMap<String, Record>,
}

impl<'llm> LLMChain<'llm> {
    /// Initialize a new LLMChain with an LLM. The LLM must implement the LLM trait.
    pub fn new(name: Option<&str>, llm: &'llm impl Generate, prompt: PromptTemplate<'llm>) -> LLMChain<'llm> {
        LLMChain {
            name: name.unwrap_or(&uuid::Uuid::new_v4().to_string()).to_string(),
            llm,
            prompt,
            records: HashMap::new(),
        }
    }

    /// Change the LLM used by the LLMChain.
    pub fn with_llm(mut self, llm: &'llm impl Generate) -> Self {
        self.llm = llm;
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
impl<'llm> ExecuteWithRecord for LLMChain<'llm> {
    async fn execute_with_record(&mut self, record_name: &str) -> Result<String, LLMError> {
        let mut h = HashMap::<String, String>::new();
        h.insert(record_name.to_string(), self.records[record_name].content.to_string());
        let prompt = self.prompt.render_data(&h)?;
        self.llm.generate(&prompt).await
    }
}

#[async_trait::async_trait(?Send)]
impl<'llm, T> Execute<T> for LLMChain<'llm>
where
    T: Serialize,
{
    async fn execute(&mut self, data: &T) -> Result<String, LLMError> {
        let prompt = self.prompt.render_data(data)?;
        println!("< Executing chain {:?}. >", self.get_name());
        let response = self.llm.generate(&prompt).await?;
        println!(
            "< Chain {:?} executed successfully. >\n< Response >\n{:?}",
            self.get_name(),
            response
        );
        Ok(response)
    }
}

impl<'llm> Clone for LLMChain<'llm> {
    fn clone(&self) -> Self {
        LLMChain {
            name: self.name.clone(),
            llm: self.llm.clone(),
            prompt: self.prompt.clone(),
            records: self.records.clone(),
        }
    }
}

#[macro_export]
macro_rules! chain {
    ($name:expr, $client:expr, $prompt:expr) => {
        LLMChain::new(Some($name), $client, $prompt)
    };
    ($client:expr, $prompt:expr) => {
        LLMChain::new(None, $client, $prompt)
    };
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::{
        llm::openai::client::OpenAIClient,
        prompts,
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

        let res = chain!(
            &client,
            prompts!(
                ("user", "What is the capital of {{country1}}"),
                ("ai", "Paris"),
                ("user", "What is the capital of {{country2}}")
            )
        )
        .execute(&Data {
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
        let res = chain!(
            &client,
            prompts!(("system", "Give a long summary of the following story:\n{{story}}"))
        )
        .with_record("story", record)
        .execute_with_record("story")
        .await
        .unwrap();
        assert!(res.contains("elephant"));
    }
}
