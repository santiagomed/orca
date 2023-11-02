use super::Pipeline;
use super::PipelineResult;
use crate::llm::LLM;
use crate::memory::Memory;
use crate::prompt::TemplateEngine;

use anyhow::Result;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Represents the simples pipeline for a Large Language Model (LLM).
///
/// This simple pipeline just takes a prompt/template and generates a response using the LLM.
/// It can make use of context, memory, and a prompt template.
pub struct LLMPipeline {
    /// The unique identifier for this LLMPipeline.
    pub name: String,

    /// The prompt template engine instance that is used by the LLMPipeline
    /// to generate the actual prompts based on the given context.
    pub template_engine: TemplateEngine,

    /// A reference to the LLM that this pipeline will use to process the prompts.
    llm: Arc<dyn LLM>,

    /// Memory associated with the LLMPipeline. It can be used to persist
    /// state or data across different executions of the pipeline.
    memory: Option<Arc<Mutex<dyn Memory>>>,

    /// The context containing key-value pairs which the `prompt`
    /// template engine might use to render the final prompt.
    context: HashMap<String, JsonValue>,
}

impl LLMPipeline {
    /// Creates a new LLMPipeline given an LLM and a prompt template.
    ///
    /// # Examples
    /// ```rust
    /// use orca::llm::openai::OpenAI;
    /// use orca::llm::LLM;
    /// use orca::prompt::TemplateEngine;
    /// use orca::pipeline::simple::LLMPipeline;
    ///
    /// let client = OpenAI::new();
    /// let prompt = "Hello, LLM!";
    /// let pipeline = LLMPipeline::new(&client).with_template("my prompt", prompt);
    /// ```
    pub fn new<M: LLM + Clone + 'static>(llm: &M) -> LLMPipeline {
        LLMPipeline {
            name: uuid::Uuid::new_v4().to_string(),
            llm: Arc::new(llm.clone()),
            template_engine: TemplateEngine::new(),
            memory: None,
            context: HashMap::new(),
        }
    }

    /// Modifies the LLMPipeline's prompt template.
    ///
    /// This is a builder-style method that returns a mutable reference to `self`.
    ///
    /// # Examples
    /// ```rust
    /// use orca::llm::openai::OpenAI;
    /// use orca::llm::LLM;
    /// use orca::prompt::TemplateEngine;
    /// use orca::pipeline::simple::LLMPipeline;
    /// use orca::template;
    ///
    /// let client = OpenAI::new();
    /// let prompt = "Hello, LLM!";
    /// let mut pipeline = LLMPipeline::new(&client).with_template("my prompt", prompt);
    /// let new_prompt = "Hello, LLM! How are you?";
    /// ```
    pub fn with_template(self, name: &str, prompt: &str) -> Self {
        Self {
            template_engine: self.template_engine.register_template(name, prompt),
            ..self
        }
    }

    /// Duplicate a template with a new name and return the new template name.
    ///
    /// # Arguments
    /// * `name` - A string slice that holds the name of the template to duplicate.
    ///
    /// # Returns
    /// An optional string that holds the name of the new template if the template with the given name exists, otherwise `None`.
    ///
    /// # Example
    /// ```rust
    /// use orca::llm::openai::OpenAI;
    /// use orca::llm::LLM;
    /// use orca::prompt::TemplateEngine;
    /// use orca::pipeline::simple::LLMPipeline;
    /// use orca::template;
    ///
    /// let client = OpenAI::new();
    /// let prompt = "Hello, LLM!";
    /// let mut pipeline = LLMPipeline::new(&client).with_template("my prompt", prompt);
    /// let new_prompt = "Hello, LLM! How are you?";
    /// let new_template_name = pipeline.duplicate_template("my prompt").unwrap();
    /// let mut pipeline = pipeline.with_template(new_template_name.as_str(), new_prompt);
    /// ```
    pub fn duplicate_template(&mut self, name: &str) -> Option<String> {
        let template_name = format!("{}-{}", name, uuid::Uuid::new_v4());
        if let Some(template) = self.template_engine.get_template(name) {
            let mut template_clone = self.template_engine.clone();
            template_clone = template_clone.register_template(template_name.as_str(), &template);
            self.template_engine = template_clone;
        } else {
            return None;
        }
        Some(template_name)
    }

    /// Change the memory used by the LLMPipeline.
    ///
    /// This is a builder-style method that returns a mutable reference to `self`.
    ///
    /// # Examples
    /// ```rust
    /// use orca::llm::openai::OpenAI;
    /// use orca::llm::LLM;
    /// use orca::prompt::TemplateEngine;
    /// use orca::pipeline::simple::LLMPipeline;
    /// use orca::memory::ChatBuffer;
    ///
    /// let client = OpenAI::new();
    /// let prompt = "Hello, LLM!";
    /// let mut pipeline = LLMPipeline::new(&client).with_template("my prompt", prompt);
    /// let memory = ChatBuffer::new();
    /// let pipeline = pipeline.with_memory(memory);
    /// ```
    pub fn with_memory<T: Memory + 'static>(mut self, memory: T) -> Self {
        self.memory = Some(Arc::new(Mutex::new(memory)));
        self
    }
}

#[async_trait::async_trait]
impl Pipeline for LLMPipeline {
    async fn execute(&self, target: &str) -> Result<PipelineResult> {
        let prompt = self.template_engine.render_context(target, &self.context)?;

        let response = if let Some(memory) = &self.memory {
            let mut locked_memory = memory.lock().await; // Lock the memory
            let mem = locked_memory.memory();
            mem.save(prompt);
            self.llm.generate(mem.clone_prompt()).await?
        } else {
            self.llm.generate(prompt.clone_prompt()).await?
        };

        Ok(PipelineResult::new(self.name.clone()).with_llm_response(response))
    }

    fn context(&mut self) -> &mut HashMap<String, JsonValue> {
        &mut self.context
    }

    fn template_engine(&mut self) -> &mut TemplateEngine {
        &mut self.template_engine
    }
}

impl Clone for LLMPipeline {
    fn clone(&self) -> Self {
        LLMPipeline {
            name: self.name.clone(),
            llm: self.llm.clone(),
            template_engine: self.template_engine.clone(),
            memory: self.memory.clone(),
            context: self.context.clone(),
        }
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::{
        llm::openai::OpenAI,
        memory,
        prompt::context::Context,
        record::{self, Spin},
    };
    use serde::Serialize;

    #[derive(Serialize)]
    pub struct DataOne {
        country1: String,
        country2: String,
    }

    #[derive(Serialize)]
    pub struct DataTwo {
        story: String,
    }

    #[tokio::test]
    async fn test_generate() {
        let client = OpenAI::new();
        let prompt = r#"
            {{#chat}}
            {{#user}}
            What is the capital of {{country1}}?
            {{/user}}
            {{#assistant}}
            Paris
            {{/assistant}}
            {{#user}}
            What is the capital of {{country2}}?
            {{/user}}
            {{/chat}}
            "#;
        let mut pipeline = LLMPipeline::new(&client).with_template("capitals", prompt);
        pipeline
            .load_context(
                &Context::new(DataOne {
                    country1: "France".to_string(),
                    country2: "Germany".to_string(),
                })
                .unwrap(),
            )
            .await;
        let res = pipeline.execute("capitals").await.unwrap().content();

        assert!(res.contains("Berlin") || res.contains("berlin"));
    }

    #[tokio::test]
    async fn test_generate_with_record() {
        let client = OpenAI::new().with_model("gpt-3.5-turbo-16k");
        let record = record::html::HTML::from_url("https://www.orwellfoundation.com/the-orwell-foundation/orwell/essays-and-other-works/shooting-an-elephant/")
            .await
            .unwrap()
            .with_selectors("p")
            .spin()
            .unwrap();

        let prompt = r#"
            {{#chat}}
            {{#user}}
            Give a long summary of the following story: {{story}}
            {{/user}}
            {{/chat}}
            "#;

        let mut pipeline = LLMPipeline::new(&client).with_template("summary", prompt);

        pipeline.load_record("story", record);
        let res = pipeline.execute("summary").await.unwrap().content();
        assert!(res.contains("elephant") || res.contains("burma"));
    }

    #[tokio::test]
    async fn test_generate_with_memory() {
        let client = OpenAI::new();

        let prompt = "{{#chat}}{{#user}}My name is Orca{{/user}}{{/chat}}";
        let pipeline = LLMPipeline::new(&client).with_template("name", prompt).with_memory(memory::ChatBuffer::new());
        pipeline.execute("name").await.unwrap();
        let pipeline = pipeline.with_template("name", "{{#chat}}{{#user}}What is my name?{{/user}}{{/chat}}");
        let res = pipeline.execute("name").await.unwrap().content();
        assert!(res.to_lowercase().contains("orca"));
    }
}
