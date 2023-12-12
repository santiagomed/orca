use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Serialize, Deserialize, Debug)]
pub struct Payload {
    input: String,
    model: String,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Response {
    object: String,
    model: String,
    data: Vec<Embedding>,
    usage: Usage,
}

impl Response {
    /// Convert the embedding response to a vector of f32 values
    pub fn to_vec(&self) -> Vec<f32> {
        match self.data.first() {
            Some(embedding) => embedding.embedding.clone(),
            None => vec![],
        }
    }
}

impl Display for Response {
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

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Usage {
    prompt_tokens: i32,
    completion_tokens: Option<i32>,
    total_tokens: i32,
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
            model: "text-embedding-ada-002".to_string(),
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

    /// Set emedding model to use
    /// e.g. "text-embedding-ada-002"
    pub fn with_model(mut self, model: &str) -> Self {
        self.model = model.to_string();
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

    /// Generate a request for the OpenAI API to create embeddings

    pub fn generate_request(&self, prompt: &str) -> Result<reqwest::Request> {
        let payload = Payload {
            model: self.model.clone(),
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

    async fn generate_embedding(&self, prompt: String) -> Result<Response> {
        let req = self.generate_request(&prompt)?;
        let res = self.client.execute(req).await?;
        let res = res.json::<Response>().await?;

        Ok(res)
    }

    async fn generate_embeddings(&self, prompts: Vec<String>) -> Result<Vec<Response>> {
        let num_prompts = prompts.len();
        let mut embeddings = vec![Response::default(); num_prompts];

        let (sender, mut receiver) = tokio::sync::mpsc::channel(num_prompts);

        for (i, prompt) in prompts.into_iter().enumerate() {
            let sender = sender.clone();
            let client = self.client.clone();
            let req = self.generate_request(&prompt)?;

            tokio::spawn(async move {
                let result = async {
                    let res = client.execute(req).await.map_err(|e| format!("Failed to execute request: {}", e))?;
                    let response = match res.json::<Response>().await {
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
                    return Err(anyhow::anyhow!("Failed to generate embedding index {}: {}", i, e));
                }
            }
        }

        Ok(embeddings)
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
