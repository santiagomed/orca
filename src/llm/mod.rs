pub mod bert;
pub mod openai;
pub mod request;

use std::{fmt::Display, sync::Arc};

use anyhow::Result;
use async_openai::types::CreateChatCompletionResponse;
use candle_core::{Device, Result as CandleResult, Tensor};

use crate::prompt::Prompt;

/// Generate with context trait is used to execute an LLM using a context and a prompt template.
/// The context is a previously created context using the Context struct. The prompt template
/// is a previously created prompt template using the template! macro.
#[async_trait::async_trait(?Send)]
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
    /// use orca::llm::openai::OpenAIClient;
    /// use orca::prompt::TemplateEngine;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///    let prompt = template!(
    ///       r#"
    ///       {{#chat}}
    ///       {{#user}}
    ///       What is the capital of France?
    ///       {{/user}}
    ///       {{/chat}}
    ///       "#
    ///    );
    ///    let client = OpenAIClient::new();
    ///    let prompt = prompt.render().unwrap();
    ///    let response = client.generate(prompt).await.unwrap();
    ///    assert!(response.to_string().to_lowercase().contains("paris"));
    /// }
    /// ```
    async fn generate(&self, prompt: Box<dyn Prompt>) -> Result<LLMResponse>;

    /// Convert an LLM into an Arc<dyn LLM> trait object.
    /// # Arguments
    /// * `self` - The LLM object to convert.
    ///
    /// # Returns
    /// * An `Arc<dyn LLM>` trait object.
    ///
    /// # Bounds
    /// * `Self: Sized` - The LLM object must be sized.
    fn into_arc(self) -> Arc<dyn LLM>
    where
        Self: Sized + 'static,
    {
        Arc::new(self)
    }
}

#[derive(Debug)]
pub enum LLMResponse {
    /// OpenAI response
    OpenAI(CreateChatCompletionResponse),

    /// Bert response
    Bert(Vec<Tensor>),

    /// Empty response; usually used to initialize a chain result when
    /// no response is available.
    Empty,
}

impl From<CreateChatCompletionResponse> for LLMResponse {
    /// Convert an OpenAI response to an LLMResponse
    fn from(response: CreateChatCompletionResponse) -> Self {
        LLMResponse::OpenAI(response)
    }
}

impl LLMResponse {
    /// Get the role of the response from an LLMResponse, if supported by the LLM.
    pub fn get_role(&self) -> String {
        match self {
            LLMResponse::OpenAI(response) => response.choices[0].message.role.to_string(),
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
                write!(f, "{}", response.choices[0].message.content.as_ref().unwrap())
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

impl Default for LLMResponse {
    /// Default LLMResponse is Empty
    fn default() -> Self {
        LLMResponse::Empty
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
