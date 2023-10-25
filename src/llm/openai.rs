use std::fmt::Display;

use crate::{
    llm::{EMBEDDING, LLM},
    prompt::{chat::Message, Prompt},
    record::{Content, Record},
};
use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::EmbeddingResponse;
use super::LLMResponse;

#[derive(Serialize, Deserialize, Debug)]
pub struct Payload {
    model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    prompt: Option<String>,
    max_tokens: i32,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<Vec<String>>,
    messages: Vec<Message>,
    stream: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EmbeddingPayload {
    input: String,
    model: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Response {
    id: String,
    object: String,
    created: i32,
    model: String,
    usage: Usage,
    choices: Vec<Choice>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OpenAIEmbeddingResponse {
    id: String,
    object: String,
    model: String,
    data: Embedding,
    usage: Usage,
}

impl OpenAIEmbeddingResponse {
    /// Convert the embedding response to a vector of f32 values
    pub fn to_vec(&self) -> Vec<f32> {
        self.data.embedding.clone()
    }

    pub fn to_string(&self) -> String {
        self.data.object.clone()
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct Embedding {
    pub index: u32,
    pub object: String,
    pub embedding: Vec<f32>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct EmbeddingUsage {
    pub prompt_tokens: u32,
    pub total_tokens: u32,
}

impl Display for Response {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = String::new();
        for choice in &self.choices {
            s.push_str(&choice.message.content);
        }
        write!(f, "{}", s)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Usage {
    prompt_tokens: i32,
    completion_tokens: i32,
    total_tokens: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Choice {
    index: i32,
    message: Message,
    finish_reason: String,
}

static OPENAI_COMPLETIONS_URL: &str = "https://api.openai.com/v1/chat/completions";
static OPENAI_EMBEDDING_URL: &str = " https://api.openai.com/v1/embeddings";

pub struct OpenAI {
    /// Client member for the OpenAI API. This client is a wrapper around the async-openai crate, with additional functionality to
    /// support LLM orchestration.
    client: Client,

    url: String,

    api_key: String,

    /// ID of the model to use.
    /// See the [model endpoint compatibility](https://platform.openai.com/docs/models/model-endpoint-compatibility) table for details on which models work with the Chat API.
    model: String,

    /// ID of the emedding model to use.
    emedding_model: String,
    /// See the [model endpoint compatibility](https://platform.openai.com/docs/models/model-endpoint-compatibility) table for details on which models work with the Chat API.

    /// What sampling temperature to use, between 0 and 2. Higher values like 0.8 will make the output more random,
    /// while lower values like 0.2 will make it more focused and deterministic.
    ///
    /// We generally recommend altering this or `top_p` but not both.
    temperature: f32, // min: 0, max: 2, default: 1,

    /// An alternative to sampling with temperature, called nucleus sampling,
    /// where the model considers the results of the tokens with top_p probability mass.
    /// So 0.1 means only the tokens comprising the top 10% probability mass are considered.
    ///
    ///  We generally recommend altering this or `temperature` but not both.
    top_p: f32, // min: 0, max: 1, default: 1

    /// If set, partial message deltas will be sent, like in ChatGPT.
    /// Tokens will be sent as data-only [server-sent events](https://developer.mozilla.org/en-US/docs/Web/API/Server-sent_events/Using_server-sent_events#Event_stream_format) as they become available,
    /// with the stream terminated by a `data: [DONE]` message.[Example Python code](https://github.com/openai/openai-cookbook/blob/main/examples/How_to_stream_completions.ipynb).
    stream: bool,

    /// The maximum number of [tokens](https://platform.openai.com/tokenizer) to generate in the chat completion.
    ///
    /// The total length of input tokens and generated tokens is limited by the model's context length. [Example Python code](https://github.com/openai/openai-cookbook/blob/main/examples/How_to_count_tokens_with_tiktoken.ipynb) for counting tokens.
    max_tokens: u16,
}

impl Default for OpenAI {
    fn default() -> Self {
        Self {
            client: Client::new(),
            url: OPENAI_COMPLETIONS_URL.to_string(),
            api_key: std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set"),
            model: "gpt-3.5-turbo".to_string(),
            emedding_model: "text-embedding-ada-002".to_string(),
            temperature: 1.0,
            top_p: 1.0,
            stream: false,
            max_tokens: 1024u16,
        }
    }
}

impl OpenAI {
    /// Create a new OpenAI client
    pub fn new() -> Self {
        Self::default()
    }

    /// Set model to use
    /// e.g. "davinci", "gpt-3.5-turbo"
    pub fn with_model(mut self, model: &str) -> Self {
        self.model = model.to_string();
        self
    }

    /// Set emedding model to use
    /// e.g. "text-embedding-ada-002"
    pub fn with_emedding_model(mut self, emedding_model: &str) -> Self {
        self.emedding_model = emedding_model.to_string();
        self
    }

    /// What sampling temperature to use, between 0 and 2. Higher values like 0.8 will make the output more random,
    /// while lower values like 0.2 will make it more focused and deterministic.
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature;
        self
    }

    /// An alternative to sampling with temperature, called nucleus sampling,
    /// where the model considers the results of the tokens with top_p probability mass.
    pub fn with_top_p(mut self, top_p: f32) -> Self {
        self.top_p = top_p;
        self
    }

    /// If set, partial message deltas will be sent, like in ChatGPT.
    pub fn with_stream(mut self, stream: bool) -> Self {
        self.stream = stream;
        self
    }

    /// The maximum number of [tokens](https://platform.openai.com/tokenizer) to generate in the chat completion.
    pub fn with_max_tokens(mut self, max_tokens: u16) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// Generate a request for the OpenAI API and set the parameters
    pub fn generate_request(&self, messages: &[Message]) -> Result<reqwest::Request> {
        let payload = Payload {
            model: self.model.clone(),
            prompt: None,
            max_tokens: self.max_tokens as i32,
            temperature: self.temperature,
            stop: None,
            messages: messages.to_vec(),
            stream: self.stream,
        };
        let req = self
            .client
            .post(&self.url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&payload)
            .build()?;
        Ok(req)
    }

    /// Generate a request for the OpenAI API to create embeddings
    pub fn generate_embedding_request(&self, input: &Record) -> Result<reqwest::Request> {
        let payload = EmbeddingPayload {
            model: self.emedding_model.clone(),
            input: input.content.to_string(),
        };

        println!("payload: {}", serde_json::to_string(&payload).unwrap());

        let req = self
            .client
            .post(OPENAI_EMBEDDING_URL)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&payload)
            .build()?;

        println!("req: {:?}", req);

        Ok(req)
    }
}

#[async_trait::async_trait]
impl LLM for OpenAI {
    async fn generate(&self, prompt: Box<dyn Prompt>) -> Result<LLMResponse> {
        let messages = prompt.to_chat()?;
        let req = self.generate_request(&messages)?;
        let res = self.client.execute(req).await?;
        let res = res.json::<Response>().await?;
        Ok(res.into())
    }
}

#[async_trait::async_trait]
impl EMBEDDING for OpenAI {
    async fn generate_embedding<'a>(&'a self, input: &'a Record) -> Result<OpenAIEmbeddingResponse> {
        println!("generate_embedding");
        let req = self.generate_embedding_request(input)?;
        println!("req: {:?}", req);
        let res = self.client.execute(req).await?;
        let res = res.json::<OpenAIEmbeddingResponse>().await?;

        Ok(res.into())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::prompt::TemplateEngine;
    use crate::template;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_generate() {
        let client = OpenAI::new();
        let mut context = HashMap::new();
        context.insert("country1", "France");
        context.insert("country2", "Germany");
        let prompt = template!(
            r#"
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
            "#
        );
        let prompt = prompt.render_context(&context).unwrap();
        let response = client.generate(prompt).await.unwrap();
        assert!(response.to_string().to_lowercase().contains("berlin"));
    }

    #[tokio::test]
    async fn test_embeddings() {
        println!("test_embeddings");
        let client = OpenAI::new();
        let content: Content = Content::String("This is a test".to_string());
        let record = Record::new(content);
        let res = client.generate_embedding(&record).await.unwrap();
        assert!(res.data.embedding.len() > 0);
    }
}
