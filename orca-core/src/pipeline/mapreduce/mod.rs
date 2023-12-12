use std::{collections::HashMap, sync::Arc};

use master::Master;
use tokio::sync::RwLock;

use crate::record::Record;

use self::task::Task;

use super::{pipeline::LLMPipeline, Pipeline, PipelineResult};
use anyhow::Result;
use serde_json::Value as JsonValue;

pub mod master;
pub mod task;
pub mod worker;

pub struct MapReducePipeline {
    context: HashMap<String, JsonValue>,
    map_pipeline: Arc<RwLock<LLMPipeline>>,
    reduce_pipeline: Arc<RwLock<LLMPipeline>>,
    records: Vec<(String, Record)>,
}

impl MapReducePipeline {
    pub fn new(map_pipeline: Arc<RwLock<LLMPipeline>>, reduce_pipeline: Arc<RwLock<LLMPipeline>>) -> Self {
        Self {
            context: HashMap::new(),
            map_pipeline,
            reduce_pipeline,
            records: Vec::new(),
        }
    }

    pub fn with_record(mut self, record_name: String, record: Record) -> Self {
        self.records.push((record_name, record));
        self
    }
}

#[async_trait::async_trait]
impl Pipeline for MapReducePipeline {
    async fn execute(&self, target: &str) -> Result<PipelineResult> {
        let task = Task::new(target.to_string(), self.records.clone());
        Ok(Master::new(
            self.records.len(),
            self.map_pipeline.clone(),
            self.reduce_pipeline.clone(),
        )
        .map(task)
        .await
        .reduce(target.to_string())
        .await)
    }

    fn context(&mut self) -> &mut HashMap<String, JsonValue> {
        &mut self.context
    }

    async fn load_context<T>(&mut self, context: &T)
    where
        T: serde::Serialize + Sync,
    {
        self.map_pipeline.write().await.load_context(context).await;
        self.reduce_pipeline.write().await.load_context(context).await;
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        llm::openai::OpenAI,
        pipelines::pipeline::LLMPipeline,
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
        let map_pipeline = Arc::new(RwLock::new(LLMPipeline::new(&client).with_template(
            "mapreduce",
            r#"{{#chat}}
                {{#user}}
                Get me a summary of the following:
                {{rec}}
                {{/user}}
                {{/chat}}
                "#,
        )));
        let reduce_pipeline = Arc::new(RwLock::new(LLMPipeline::new(&client).with_template(
            "mapreduce",
            r#"{{#chat}}
            {{#user}}
            Get me a summary of the following:
            {{rec}}
            {{/user}}
            {{/chat}}
            "#,
        )));
        let mp_pipeline = MapReducePipeline::new(map_pipeline, reduce_pipeline)
            .with_record("rec".to_string(), split_rec[0].clone())
            .with_record("rec".to_string(), split_rec[1].clone())
            .with_record("rec".to_string(), split_rec[2].clone())
            .with_record("rec".to_string(), split_rec[3].clone())
            .with_record("rec".to_string(), split_rec[4].clone())
            .execute("mapreduce")
            .await;
        assert!(mp_pipeline.is_ok())
    }
}
