use super::{Pipeline, PipelineResult};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct SequentialPipeline<P> {
    /// The name of the LLMPipeline.
    name: String,

    /// Vector of LLM pipelines used by the SequentialPipeline.
    pipelines: Vec<Arc<RwLock<P>>>,
}

impl<P> Default for SequentialPipeline<P> {
    fn default() -> Self {
        Self {
            name: uuid::Uuid::new_v4().to_string(),
            pipelines: Vec::new(),
        }
    }
}

impl<P: Pipeline> SequentialPipeline<P> {
    /// Initialize a new sequential pipeline.
    pub fn new() -> SequentialPipeline<P> {
        SequentialPipeline::default()
    }

    /// Add a simple LLM Pipeline to the sequential pipeline.
    pub fn link(mut self, pipeline: P) -> SequentialPipeline<P> {
        self.pipelines.push(Arc::new(RwLock::new(pipeline)));
        self
    }
}

#[async_trait::async_trait]
impl<P: Pipeline> Pipeline for SequentialPipeline<P> {
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
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::{llm::openai::OpenAI, pipeline::simple::LLMPipeline, prompt::context::Context};
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

        let context = &Context::new(&Data {
            play: "Hamlet".to_string(),
        })
        .unwrap();
        let summary = LLMPipeline::new(&client).load_template("review", first).unwrap().load_context(context).unwrap();
        let review = LLMPipeline::new(&client).load_template("review", second).unwrap().load_context(context).unwrap();

        let pipeline = SequentialPipeline::new().link(summary).link(review);
        let res = pipeline.execute("review").await;
        assert!(res.is_ok());
    }
}
