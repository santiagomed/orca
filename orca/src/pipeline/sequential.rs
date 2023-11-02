use crate::prompt::context::Context;

use super::simple::LLMPipeline;
use super::{Pipeline, PipelineResult};
use anyhow::Result;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct SequentialPipeline {
    /// The name of the LLMPipeline.
    name: String,

    /// Vector of LLM pipelines used by the SequentialPipeline.
    pipelines: Vec<Arc<RwLock<dyn Pipeline>>>,

    /// The context for for the templates used by the SequentialPipeline.
    context: HashMap<String, JsonValue>,
}

impl Default for SequentialPipeline {
    fn default() -> Self {
        Self {
            name: uuid::Uuid::new_v4().to_string(),
            pipelines: Vec::new(),
            context: HashMap::new(),
        }
    }
}

impl SequentialPipeline {
    /// Initialize a new sequential pipeline.
    pub fn new() -> SequentialPipeline {
        SequentialPipeline::default()
    }

    /// Add a simple LLM Pipeline to the sequential pipeline.
    pub fn link(mut self, pipeline: LLMPipeline) -> SequentialPipeline {
        self.pipelines.push(Arc::new(RwLock::new(pipeline)));
        self
    }
}

#[async_trait::async_trait]
impl Pipeline for SequentialPipeline {
    async fn execute(&self, target: &str) -> Result<PipelineResult> {
        let mut response = String::new();
        let mut result: PipelineResult = PipelineResult::new(self.name.to_string()); // initialize result to a default value
        for pipeline in &self.pipelines {
            if !response.is_empty() {
                pipeline
                    .write()
                    .await
                    .template_engine()
                    .add_to_template(target, &format!("{{{{#user}}}}{}{{{{/user}}}}", response));
            }
            result = pipeline.read().await.execute(target).await?;
            response = result.content();
        }
        Ok(result)
    }

    fn context(&mut self) -> &mut HashMap<String, JsonValue> {
        &mut self.context
    }

    async fn load_context(&mut self, context: &Context) {
        for pipeline in &mut self.pipelines {
            pipeline.write().await.load_context(context).await;
        }
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::llm::openai::OpenAI;
    use serde::Serialize;

    #[derive(Serialize)]
    pub struct Data {
        play: String,
    }

    #[tokio::test]
    async fn test_generate() {
        let client = OpenAI::new();

        let first = "{{#chat}}{{#user}}Give me a summary of {{play}}'s plot.{{/user}}{{/chat}}";
        let second = "{{#chat}}{{#system}}You are a professional critic. When given a summary of a play, you must write a review of it. Here is a summary of {{play}}'s plot:{{/system}}{{/chat}}";

        let mut pipeline = SequentialPipeline::new()
            .link(LLMPipeline::new(&client).with_template("review", first))
            .link(LLMPipeline::new(&client).with_template("review", second));
        pipeline
            .load_context(
                &Context::new(&Data {
                    play: "Hamlet".to_string(),
                })
                .unwrap(),
            )
            .await;
        let res = pipeline.execute("review").await;
        assert!(res.is_ok());
    }
}
