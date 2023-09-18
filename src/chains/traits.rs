use crate::llm::error::LLMError;

#[async_trait::async_trait(?Send)]
pub trait Execute<T> {
    /// Execute an LLM chain using a context and a prompt template.
    async fn execute(&mut self, data: &T) -> Result<String, LLMError>;
}

#[async_trait::async_trait(?Send)]
pub trait ExecuteWithRecord {
    /// Execute an LLM chain using a record and a prompt template.
    ///
    /// [TODO]:
    /// Creating a new hashmap when we already have one with records is not clean code
    /// since we created the record object to standardize the way we handle records.
    async fn execute_with_record(&mut self, record_name: &str) -> Result<String, LLMError>;
}
