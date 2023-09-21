pub mod chain;
pub mod sequential;

use std::collections::HashMap;

use serde::Serialize;

use crate::{llm::error::LLMError, record::record::Record};

#[async_trait::async_trait(?Send)]
pub trait Chain {
    /// Execute an LLM chain using a context and a prompt template.
    async fn execute(&mut self) -> Result<String, LLMError>;

    /// Set the context of the LLMChain.
    fn set_context<T>(&mut self, context: &T)
    where
        T: Serialize,
    {
        let context = serde_json::to_value(context).unwrap();
        let context = context.as_object().unwrap();
        for (key, value) in context {
            self.get_context().insert(key.to_string(), value.to_string());
        }
    }

    /// Save a record content to the context of an LLM Chain.
    fn set_record(&mut self, name: &str, record: Record) {
        let context = self.get_context();
        if !context.contains_key(name) {
            context.insert(name.to_string(), record.content.to_string());
        }
    }

    fn get_context(&mut self) -> &mut HashMap<String, String>;
}
