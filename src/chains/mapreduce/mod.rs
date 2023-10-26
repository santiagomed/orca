use std::{collections::HashMap, sync::Arc};

use master::Master;
use tokio::sync::RwLock;

use crate::record::Record;

use self::task::Task;

use super::{chain::LLMChain, Chain, ChainResult};
use anyhow::Result;

pub mod master;
pub mod task;
pub mod worker;

pub struct MapReduceChain {
    context: HashMap<String, String>,
    map_chain: Arc<RwLock<LLMChain>>,
    reduce_chain: Arc<RwLock<LLMChain>>,
    records: Vec<(String, Record)>,
}

impl MapReduceChain {
    pub fn new(map_chain: Arc<RwLock<LLMChain>>, reduce_chain: Arc<RwLock<LLMChain>>) -> Self {
        Self {
            context: HashMap::new(),
            map_chain,
            reduce_chain,
            records: Vec::new(),
        }
    }

    pub fn with_record(mut self, record_name: String, record: Record) -> Self {
        self.records.push((record_name, record));
        self
    }
}

#[async_trait::async_trait]
impl Chain for MapReduceChain {
    async fn execute(&self, target: &str) -> Result<ChainResult> {
        let task = Task::new(target.to_string(), self.records.clone());
        Ok(
            Master::new(self.records.len(), self.map_chain.clone(), self.reduce_chain.clone())
                .map(task)
                .await
                .reduce(target.to_string())
                .await,
        )
    }

    fn context(&mut self) -> &mut HashMap<String, String> {
        &mut self.context
    }

    async fn load_context<T>(&mut self, context: &T)
    where
        T: serde::Serialize + Sync,
    {
        self.map_chain.blocking_write().load_context(context).await;
        self.reduce_chain.blocking_write().load_context(context).await;
    }
}

#[cfg(test)]
mod tests {
    use crate::{chains::chain::LLMChain, llm::openai::OpenAI};

    use super::*;

    #[tokio::test]
    #[ignore = "wip"]
    async fn test_mapreduce() {
        let client = Arc::new(OpenAI::new());
        let map_chain = Arc::new(RwLock::new(
            LLMChain::new(client.clone()).with_prompt("mapreduce", "Hello, {name}!"),
        ));
        let reduce_chain = Arc::new(RwLock::new(
            LLMChain::new(client.clone()).with_prompt("mapreduce", "Hello, {name}!"),
        ));
        let mp_chain = MapReduceChain::new(map_chain, reduce_chain).execute("mapreduce").await;
        assert!(mp_chain.is_ok())
    }
}
