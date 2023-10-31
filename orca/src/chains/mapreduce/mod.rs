use std::{collections::HashMap, sync::Arc};

use master::Master;
use tokio::sync::RwLock;

use crate::record::Record;

use self::task::Task;

use super::{chain::LLMChain, Chain, ChainResult};
use anyhow::Result;
use serde_json::Value as JsonValue;

pub mod master;
pub mod task;
pub mod worker;

pub struct MapReduceChain {
    context: HashMap<String, JsonValue>,
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

    fn context(&mut self) -> &mut HashMap<String, JsonValue> {
        &mut self.context
    }

    async fn load_context<T>(&mut self, context: &T)
    where
        T: serde::Serialize + Sync,
    {
        self.map_chain.write().await.load_context(context).await;
        self.reduce_chain.write().await.load_context(context).await;
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        chains::chain::LLMChain,
        llm::openai::OpenAI,
        record::{pdf::Pdf, Spin},
    };

    use super::*;

    #[tokio::test]
    #[ignore = "wip"]
    async fn test_mapreduce() {
        std::env::set_var("STANDARD_FONTS", "./assets/pdf_fonts");
        let rec = Pdf::from_file("./tests/memgpt.pdf", false).spin().unwrap();
        let split_rec = rec.split(5);

        let client = OpenAI::new();
        let map_chain = Arc::new(RwLock::new(LLMChain::new(&client).with_template(
            "mapreduce",
            r#"{{#chat}}
                {{#user}}
                Get me a summary of the following:
                {{rec}}
                {{/user}}
                {{/chat}}
                "#,
        )));
        let reduce_chain = Arc::new(RwLock::new(LLMChain::new(&client).with_template(
            "mapreduce",
            r#"{{#chat}}
            {{#user}}
            Get me a summary of the following:
            {{rec}}
            {{/user}}
            {{/chat}}
            "#,
        )));
        let mp_chain = MapReduceChain::new(map_chain, reduce_chain)
            .with_record("rec".to_string(), split_rec[0].clone())
            .with_record("rec".to_string(), split_rec[1].clone())
            .with_record("rec".to_string(), split_rec[2].clone())
            .with_record("rec".to_string(), split_rec[3].clone())
            .with_record("rec".to_string(), split_rec[4].clone())
            .execute("mapreduce")
            .await;
        assert!(mp_chain.is_ok())
    }
}
