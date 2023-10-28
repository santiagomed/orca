pub mod bert;
pub mod openai;
pub mod quantized;
pub mod request;

use std::fmt::Display;

use anyhow::Result;
use candle_core::{Device, Result as CandleResult, Tensor};

use crate::prompt::Prompt;

/// Generate with context trait is used to execute an LLM using a context and a prompt template.
/// The context is a previously created context using the Context struct. The prompt template
/// is a previously created prompt template using the template! macro.
#[async_trait::async_trait]
pub trait LLM: Sync + Send {
    /// Generate a response from an LLM using a context and a prompt template.
    /// # Arguments
    /// * `prompt` - A prompt trait object.
    ///
    /// # Examples
    /// This example uses the OpenAI chat models.
    /// ```
    /// use orca::llm::LLM;
    /// use orca::prompt::Prompt;
    /// use orca::template;
    /// use orca::llm::openai::OpenAI;
    /// use orca::prompt::TemplateEngine;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///    let prompt = template!(
    ///       "my template",
    ///       r#"
    ///       {{#chat}}
    ///       {{#user}}
    ///       What is the capital of France?
    ///       {{/user}}
    ///       {{/chat}}
    ///       "#
    ///    );
    ///    let client = OpenAI::new();
    ///    let prompt = prompt.render("my template").unwrap();
    ///    let response = client.generate(prompt).await.unwrap();
    ///    assert!(response.to_string().to_lowercase().contains("paris"));
    /// }
    /// ```
    async fn generate(&self, prompt: Box<dyn Prompt>) -> Result<LLMResponse>;
}

/// Embedding trait is used to generate an embedding from an Online Service.
#[async_trait::async_trait]
pub trait Embedding {
    /// Generate an embedding from an Online Service.
    /// # Arguments
    /// * `input` - A Record trait object.
    ///
    /// # Examples
    /// This example uses the OpenAI chat models.
    /// ```
    /// use orca::prompt;
    /// use orca::llm::Embedding;
    /// use orca::record::Record;
    /// use orca::llm::openai::OpenAI;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///    let client = OpenAI::new();
    ///    let input = prompt!("Hello, world");
    ///    let response = client.generate_embedding(input).await.unwrap();
    ///    assert!(response.get_embedding().len() > 0);
    /// }
    /// ```
    async fn generate_embedding(&self, prompt: Box<dyn Prompt>) -> Result<EmbeddingResponse>;
}

#[derive(Debug)]
pub enum EmbeddingResponse {
    /// OpenAI embedding response
    OpenAI(openai::OpenAIEmbeddingResponse),

    /// Empty response; usually used to initialize a chain result when
    /// no response is available.
    Empty,
}

impl From<openai::OpenAIEmbeddingResponse> for EmbeddingResponse {
    /// Convert an OpenAI embedding response to an EmbeddingResponse
    fn from(response: openai::OpenAIEmbeddingResponse) -> Self {
        EmbeddingResponse::OpenAI(response)
    }
}

#[derive(Debug)]
pub enum LLMResponse {
    /// OpenAI response
    OpenAI(openai::Response),

    /// Bert response
    Bert(Vec<Tensor>),

    /// Quantized model response
    Quantized(String),

    /// Empty response; usually used to initialize a chain result when
    /// no response is available.
    Empty,
}

impl From<openai::Response> for LLMResponse {
    /// Convert an OpenAI response to an LLMResponse
    fn from(response: openai::Response) -> Self {
        LLMResponse::OpenAI(response)
    }
}

impl EmbeddingResponse {
    /// Get the embedding from an OpenAIEmbeddingResponse
    pub fn get_embedding(&self) -> Vec<f32> {
        match self {
            EmbeddingResponse::OpenAI(response) => response.to_vec(),
            EmbeddingResponse::Empty => Vec::new(),
        }
    }
}

impl LLMResponse {
    /// Get the role of the response from an LLMResponse, if supported by the LLM.
    pub fn get_role(&self) -> String {
        match self {
            LLMResponse::OpenAI(response) => response.to_string(),
            LLMResponse::Quantized(_) => "ai".to_string(),
            LLMResponse::Bert(_) => "ai".to_string(),
            LLMResponse::Empty => "".to_string(),
        }
    }
}

impl Display for LLMResponse {
    /// Display the response content from an LLMResponse
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LLMResponse::OpenAI(response) => {
                write!(f, "{}", response)
            }
            LLMResponse::Quantized(response) => {
                write!(f, "{}", response)
            }
            LLMResponse::Bert(response) => {
                write!(
                    f,
                    "{}",
                    response.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(", ")
                )
            }
            LLMResponse::Empty => write!(f, ""),
        }
    }
}

impl Display for EmbeddingResponse {
    /// Display the response content from an EmbeddingResponse
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmbeddingResponse::OpenAI(response) => {
                write!(f, "{}", response.to_string())
            }
            EmbeddingResponse::Empty => write!(f, ""),
        }
    }
}

impl Default for LLMResponse {
    /// Default LLMResponse is Empty
    fn default() -> Self {
        LLMResponse::Empty
    }
}

impl Default for EmbeddingResponse {
    /// Default EmbeddingResponse is Empty
    fn default() -> Self {
        EmbeddingResponse::Empty
    }
}

/// Returns a `Device` object representing either a CPU or a CUDA device.
///
/// # Arguments
/// * `cpu` - A boolean value indicating whether to use a CPU device (`true`) or a CUDA device (`false`).
///
/// # Examples
/// ```
/// use orca::llm::device;
///
/// // Use a CPU device
/// let cpu_device = device(true).unwrap();
///
/// // Use a CUDA device
/// let cuda_device = device(false).unwrap();
/// ```
pub fn device(cpu: bool) -> CandleResult<Device> {
    if cpu {
        Ok(Device::Cpu)
    } else {
        let device = Device::cuda_if_available(0)?;
        if !device.is_cuda() {
            println!("Running on CPU, to run on GPU, specify it using the llm.with_gpu() method.");
        }
        Ok(device)
    }
}

/// This is a wrapper around a tokenizer to ensure that tokens can be returned to the user in a
/// streaming way rather than having to wait for the full decoding.
pub struct TokenOutputStream {
    tokenizer: tokenizers::Tokenizer,
    tokens: Vec<u32>,
    prev_index: usize,
    current_index: usize,
}

impl TokenOutputStream {
    pub fn new(tokenizer: tokenizers::Tokenizer) -> Self {
        Self {
            tokenizer,
            tokens: Vec::new(),
            prev_index: 0,
            current_index: 0,
        }
    }

    pub fn into_inner(self) -> tokenizers::Tokenizer {
        self.tokenizer
    }

    fn decode(&self, tokens: &[u32]) -> candle_core::Result<String> {
        match self.tokenizer.decode(tokens, true) {
            Ok(str) => Ok(str),
            Err(err) => candle_core::bail!("cannot decode: {err}"),
        }
    }

    // https://github.com/huggingface/text-generation-inference/blob/5ba53d44a18983a4de32d122f4cb46f4a17d9ef6/server/text_generation_server/models/model.py#L68
    pub fn next_token(&mut self, token: u32) -> candle_core::Result<Option<String>> {
        let prev_text = if self.tokens.is_empty() {
            String::new()
        } else {
            let tokens = &self.tokens[self.prev_index..self.current_index];
            self.decode(tokens)?
        };
        self.tokens.push(token);
        let text = self.decode(&self.tokens[self.prev_index..])?;
        if text.len() > prev_text.len() && text.chars().last().unwrap().is_ascii() {
            let text = text.split_at(prev_text.len());
            self.prev_index = self.current_index;
            self.current_index = self.tokens.len();
            Ok(Some(text.1.to_string()))
        } else {
            Ok(None)
        }
    }

    pub fn decode_rest(&self) -> Result<Option<String>> {
        let prev_text = if self.tokens.is_empty() {
            String::new()
        } else {
            let tokens = &self.tokens[self.prev_index..self.current_index];
            self.decode(tokens)?
        };
        let text = self.decode(&self.tokens[self.prev_index..])?;
        if text.len() > prev_text.len() {
            let text = text.split_at(prev_text.len());
            Ok(Some(text.1.to_string()))
        } else {
            Ok(None)
        }
    }

    pub fn decode_all(&self) -> candle_core::Result<String> {
        self.decode(&self.tokens)
    }

    pub fn get_token(&self, token_s: &str) -> Option<u32> {
        self.tokenizer.get_vocab(true).get(token_s).copied()
    }

    pub fn tokenizer(&self) -> &tokenizers::Tokenizer {
        &self.tokenizer
    }

    pub fn clear(&mut self) {
        self.tokens.clear();
        self.prev_index = 0;
        self.current_index = 0;
    }
}
