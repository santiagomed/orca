use std::env;
use std::fmt::Display;

use crate::{
    llm::{Embedding as EmbeddingTrait, LLM},
    prompt::{chat::Message, Prompt},
};
use anyhow::{anyhow, bail, Result};
use log::{debug, info, error};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};

use super::{EmbeddingResponse, LLMResponse};

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
    response_format: ResponseFormatWrapper,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EmbeddingPayload {
    input: String,
    model: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseFormatWrapper {
    #[serde(rename = "type")]
    pub format: ResponseFormat,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Response {
    id: String,
    object: String,
    created: i32,
    model: String,
    usage: Usage,
    choices: Vec<Choice>,
    system_fingerprint: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct QuotaError {
    message: String,
    #[serde(rename = "type")]
    _type: String,
    param: String,
    code: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum OpenAIResponse {
    Response(Response),
    QuotaError(QuotaError),
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct OpenAIEmbeddingResponse {
    object: String,
    model: String,
    data: Vec<Embedding>,
    usage: Usage,
}

impl OpenAIEmbeddingResponse {
    /// Convert the embedding response to a vector of f32 values
    pub fn to_vec(&self) -> Vec<f32> {
        match self.data.first() {
            Some(embedding) => embedding.embedding.clone(),
            None => vec![],
        }
    }
}

impl Display for OpenAIEmbeddingResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = String::new();
        for embedding in &self.data {
            s.push_str(&embedding.object);
        }
        write!(f, "{}", s)
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Default)]
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

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Usage {
    prompt_tokens: i32,
    completion_tokens: Option<i32>,
    total_tokens: i32,
    completion_tokens_details: CompletionTokensDetails,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct CompletionTokensDetails {
    reasoning_tokens: i32,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ResponseFormat {
    Text,
    JsonObject,
}

impl From<ResponseFormat> for ResponseFormatWrapper {
    fn from(format: ResponseFormat) -> Self {
        ResponseFormatWrapper { format }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Choice {
    index: i32,
    message: Message,
    logprobs: Option<String>,
    finish_reason: String,
}

static OPENAI_COMPLETIONS_URL: &str = "https://api.openai.com/v1/chat/completions";
static OPENAI_EMBEDDING_URL: &str = " https://api.openai.com/v1/embeddings";

#[derive(Clone)]
pub struct OpenAI {
    /// Client member for the OpenAI API. This client is a wrapper around the async-openai crate, with additional functionality to
    /// support LLM orchestration.
    client: Client,

    /// URL of the OpenAI API
    /// This URL is set to https://api.openai.com/v1/chat/completions by default.
    url: String,

    /// API key for the OpenAI API
    /// This key is stored in the OPENAI_API_KEY environment variable.
    api_key: String,

    /// ID of the model to use.
    /// See the [model endpoint compatibility](https://platform.openai.com/docs/models/model-endpoint-compatibility) table for details on which models work with the Chat API.
    model: String,

    /// ID of the emedding model to use.
    /// See the [model endpoint compatibility](https://platform.openai.com/docs/models/model-endpoint-compatibility) table for details on which models work with the Chat API.
    emedding_model: String,

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

    /// The format of the returned data. With the new update, the response can be set to a JSON object.
    /// https://platform.openai.com/docs/guides/text-generation/json-mode
    response_format: ResponseFormat,
}

impl Default for OpenAI {
    fn default() -> Self {
        let api_key = match env::var("OPENAI_API_KEY") {
            Ok(api_key) => api_key,
            Err(e) => {
                let msg = format!("OPENAI_API_KEY: {e}");
                error!("{msg}");
                panic!("{msg}")
            }
        };

        Self {
            client: Client::new(),
            url: OPENAI_COMPLETIONS_URL.to_string(),
            api_key,
            model: "gpt-3.5-turbo-1106".to_string(),
            emedding_model: "text-embedding-ada-002".to_string(),
            temperature: 1.0,
            top_p: 1.0,
            stream: false,
            max_tokens: 1024u16,
            response_format: ResponseFormat::Text,
        }
    }
}

impl OpenAI {
    /// Create a new OpenAI client
    pub fn new() -> Self {
        Self::default()
    }

    /// Set Openai Api Key
    /// e.g. `sk-zv62KQG06YS4HSE13VJVTa01J86PDSWMS2V775BHGEBY48GD`
    pub fn with_api_key(mut self, api_key: &str) -> Self {
        self.api_key = api_key.to_string();
        self
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

    pub fn with_response_format(mut self, response_format: ResponseFormat) -> Self {
        self.response_format = response_format;
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
            response_format: self.response_format.clone().into(),
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

    pub fn generate_embedding_request(&self, prompt: &str) -> Result<reqwest::Request> {
        let payload = EmbeddingPayload {
            model: self.emedding_model.clone(),
            input: prompt.to_string(),
        };

        let req = self
            .client
            .post(OPENAI_EMBEDDING_URL)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&payload)
            .build()?;

        Ok(req)
    }
}

#[async_trait::async_trait]
impl LLM for OpenAI {
    async fn generate(&self, prompt: Box<dyn Prompt>) -> Result<LLMResponse> {
        let messages = prompt.to_chat()?;
        let req = self.generate_request(messages.to_vec_ref())?;
        let res = self.client.execute(req).await?;

        let status = res.status();

        if status.is_success() {
            let bytes = res.bytes().await?;
            match serde_json::from_slice::<Response>(&bytes) {
                Ok(response) => Ok(response.into()),
                Err(e) => {
                    let msg = format!("Erro unknown: {}, for this response: {}",
                                      e,
                                      std::str::from_utf8(&bytes).unwrap_or("<invalid utf-8>"));
                    error!("{}", msg);
                    bail!("error: {}, req payload: {}", e, msg);
                }
            }
        } else if status == StatusCode::TOO_MANY_REQUESTS {
            let bytes = res.bytes().await?;
            match serde_json::from_slice::<QuotaError>(&bytes) {
                Ok(quota_error) => bail!("Quota error: {}", quota_error.message),
                Err(e) => {
                    let msg = format!("Erro unknown: {}, for this response: {}",
                                      e,
                                      std::str::from_utf8(&bytes).unwrap_or("<invalid utf-8>"));
                    error!("{}", msg);
                    bail!("error: {}, req payload: {}", e, msg);
                }
            }
        } else {
            let msg = format!("api response with status code: {}, payloads: {}", status, res.text().await?);
            error!("{}", msg);
            bail!(msg)
        }
    }
}

const MAX_RETRIES: u32 = 5;
const INITIAL_BACKOFF: tokio::time::Duration = tokio::time::Duration::from_millis(100);
const MAX_BACKOFF: tokio::time::Duration = tokio::time::Duration::from_secs(10);

async fn send_with_exponential_backoff<T>(sender: &tokio::sync::mpsc::Sender<T>, message: T) -> Result<(), String>
where
    T: Clone + Send + 'static,
{
    let mut attempts = 0;
    let mut backoff = INITIAL_BACKOFF;

    loop {
        match sender.send(message.clone()).await {
            Ok(_) => return Ok(()),
            Err(_) if attempts < MAX_RETRIES => {
                attempts += 1;
                tokio::time::sleep(backoff).await;
                backoff = std::cmp::min(backoff * 2, MAX_BACKOFF);
            }
            Err(e) => return Err(format!("Failed to send message: {}", e)),
        }
    }
}

#[async_trait::async_trait]
impl EmbeddingTrait for OpenAI {
    async fn generate_embedding(&self, prompt: Box<dyn Prompt>) -> Result<EmbeddingResponse> {
        let req = self.generate_embedding_request(&prompt.to_string())?;
        debug!("req: {:?}", req);

        let res = self.client.execute(req).await?;
        debug!("res: {:?}", res);

        let status_code = res.status();
        debug!("status_code: {:?}", status_code);

        if !status_code.is_success() {
            let msg = format!(r#"{{ "status_code": "{}", "payload": {} }}"#, status_code, res.text().await?);
            error!("{}", msg);
            bail!(msg);
        }

        match res.json::<OpenAIEmbeddingResponse>().await {
            Ok(res) => {
                Ok(res.into())
            }
            Err(e) => {
                error!(r#"{{ "error": "{}", "stack_trace": ["{}"] }}"#, "Error parsing response from openai", e);
                bail!(e)
            }
        }
    }

    async fn generate_embeddings(&self, prompts: Vec<Box<dyn Prompt>>) -> Result<EmbeddingResponse> {
        let num_prompts = prompts.len();
        let mut embeddings = vec![OpenAIEmbeddingResponse::default(); num_prompts];

        let (sender, mut receiver) = tokio::sync::mpsc::channel(num_prompts);

        for (i, prompt) in prompts.into_iter().enumerate() {
            let sender = sender.clone();
            let client = self.client.clone();
            let req = self.generate_embedding_request(&prompt.to_string())?;

            tokio::spawn(async move {
                let result = async {
                    let res = client.execute(req).await.map_err(|e| format!("Failed to execute request: {}", e))?;
                    let response = match res.json::<OpenAIEmbeddingResponse>().await {
                        Ok(response) => response,
                        Err(e) => {
                            return Err(format!("Failed to parse response: {}", e));
                        }
                    };
                    Ok(response)
                }
                    .await;

                // Send back the result (success or error) via the channel.
                if let Err(e) = send_with_exponential_backoff(&sender, (i, result)).await {
                    // Log the error or take appropriate action
                    log::error!("Error sending message: {}", e);
                }
            });
        }

        drop(sender);

        while let Some((i, result)) = receiver.recv().await {
            match result {
                Ok(response) => {
                    embeddings[i] = response;
                }
                Err(e) => {
                    return Err(anyhow!("Failed to generate embedding index {}: {}", i, e));
                }
            }
        }

        Ok(EmbeddingResponse::OpenAI(embeddings))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::prompt::TemplateEngine;
    use crate::template;
    use crate::{prompt, prompts};
    use std::collections::HashMap;
    use dotenv::dotenv;


    #[test]
    fn test_parse_response() {
        let response = r#"{
          "id": "chatcmpl-A9aBeoxUbUFGGDu79XwBd3AXmyEwq",
          "object": "chat.completion",
          "created": 1726847418,
          "model": "gpt-3.5-turbo-1106",
          "choices": [
            {
              "index": 0,
              "message": {
                "role": "assistant",
                "content": "Hola, Miuler. Soy un modelo de lenguaje AI, ¡encantado de conocerte!",
                "refusal": null
              },
              "logprobs": null,
              "finish_reason": "stop"
            }
          ],
          "usage": {
            "prompt_tokens": 21,
            "completion_tokens": 22,
            "total_tokens": 43,
            "completion_tokens_details": {
              "reasoning_tokens": 0
            }
          },
          "system_fingerprint": "fp_e81b59fe66"
        }"#;

        let response = serde_json::from_str::<Response>(response);
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn test_generate() {
        let client = OpenAI::new();
        let mut context = HashMap::new();
        context.insert("country1", "France");
        context.insert("country2", "Germany");
        let prompt = template!(
            "my template",
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
        let prompt = prompt.render_context("my template", &context).unwrap();
        let response = client.generate(prompt).await.unwrap();
        assert!(response.to_string().to_lowercase().contains("berlin"));
    }

    #[tokio::test]
    async fn test_generate_json_mode() {
        let client = OpenAI::new().with_model("gpt-3.5-turbo-1106").with_response_format(ResponseFormat::JsonObject);
        let mut context = HashMap::new();
        context.insert("country1", "France");
        context.insert("country2", "Germany");
        let prompt = template!(
            "my template",
            r#"
            {{#chat}}
            {{#user}}
            What is the capital of {{country1}}?
            {{/user}}
            {{#assistant}}
            Paris
            {{/assistant}}
            {{#user}}
            What is the capital of {{country2}} in a JSON format?
            {{/user}}
            {{/chat}}
            "#
        );
        let prompt = prompt.render_context("my template", &context).unwrap();
        let response = client.generate(prompt).await.unwrap();
        assert!(response.to_string().to_lowercase().contains("berlin"));
        //  Assert response is a JSON object
        assert!(response.to_string().starts_with("{"));
    }

    #[tokio::test]
    async fn test_embedding() {
        dotenv().ok();
        env_logger::init();
        let client = OpenAI::new();
        let content = prompt!("This is a test");
        let res = client.generate_embedding(content).await.unwrap();
        assert!(res.to_vec2().unwrap().len() > 0);
    }

    #[tokio::test]
    async fn test_embeddings() {
        let client = OpenAI::new();
        let content: Vec<Box<dyn Prompt>> = prompts!("This is a test", "This is another test", "This is a third test");
        let res = client.generate_embeddings(content).await.unwrap();
        assert!(res.to_vec2().unwrap().len() > 0);
    }

    #[test]
    fn test_serde() {
        tracing_subscriber::fmt().pretty().init();

        use serde::{Serialize, Deserialize};

        #[derive(Serialize, Deserialize, Debug)]
        #[serde(tag = "type")] // Añadiendo una etiqueta para discriminar entre las variantes
        enum OpenAIResponse {
            #[serde(rename = "response")]
            Response(Response),
            #[serde(rename = "quota_error")]
            QuotaError(QuotaError),
        }

        #[derive(Serialize, Deserialize, Debug)]
        struct Response {
            field1: String,
            field2: String,
        }

        #[derive(Serialize, Deserialize, Debug)]
        struct QuotaError {
            field1: String,
            field2: String,
        }

        let json_response = r#"{ "type": "response", "field1": "value1", "field2": "value2" }"#;
        let openai_response: OpenAIResponse = serde_json::from_str(json_response).unwrap();
        info!("openai_response: {:?}", openai_response);

        let json_quota_error = r#"{ "type": "quota_error", "field1": "value1", "field2": "value2" }"#;
        let openai_quota_error: OpenAIResponse = serde_json::from_str(json_quota_error).unwrap();
        info!("openai_quota_error: {:?}", openai_quota_error);
    }
}
