use std::{collections::HashMap, sync::Arc};

use master::Master;
use tokio::sync::Mutex;

use crate::record::Record;

use self::task::Task;

use super::{Chain, ChainResult};
use crate::record;
use anyhow::Result;

pub mod master;
pub mod task;
pub mod worker;

pub struct MapReduceChain {
    context: HashMap<String, String>,
    map_chain: Arc<Mutex<dyn Chain>>,
    reduce_chain: Arc<Mutex<dyn Chain>>,
    records: Vec<(String, Record)>,
}

impl MapReduceChain {
    pub fn new(map_chain: Arc<Mutex<dyn Chain>>, reduce_chain: Arc<Mutex<dyn Chain>>) -> Self {
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
    async fn execute(&mut self) -> Result<ChainResult> {
        let task = Task::new(self.records.clone());
        Ok(
            Master::new(self.records.len(), self.map_chain.clone(), self.reduce_chain.clone())
                .map(task)
                .await
                .reduce()
                .await,
        )
    }

    fn context(&mut self) -> &mut HashMap<String, String> {
        &mut self.context
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
        let map_chain = Arc::new(Mutex::new(LLMChain::new(client.clone(), "Hello, {name}!")));
    }
}
