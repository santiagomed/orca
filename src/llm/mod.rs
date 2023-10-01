pub mod bert;
pub mod openai;
pub mod request;

use anyhow::Result;
use async_openai::types::CreateChatCompletionResponse;
use candle_core::{Device, Result as CandleResult, Tensor};

/// Generate with context trait is used to execute an LLM using a context and a prompt template.
/// The context is a previously created context using the Context struct. The prompt template
/// is a previously created prompt template using the prompt! macro.
#[async_trait::async_trait(?Send)]
pub trait LLM {
    /// Generate a response from an LLM using a context and a prompt template.
    async fn generate(&self) -> Result<LLMResponse>;
}

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
    /// Get the response content from an LLMResponse
    pub fn get_response_content(&self) -> String {
        match self {
            LLMResponse::OpenAI(response) => response.choices[0].message.content.as_ref().unwrap().to_string(),
            LLMResponse::Bert(response) => response.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(", "),
            LLMResponse::Empty => "".to_string(),
        }
    }

    /// Get the role of the response from an LLMResponse, if supported by the LLM.
    pub fn get_role(&self) -> String {
        match self {
            LLMResponse::OpenAI(response) => response.choices[0].message.role.to_string(),
            LLMResponse::Bert(_) => "ai".to_string(),
            LLMResponse::Empty => "".to_string(),
        }
    }
}

impl Default for LLMResponse {
    /// Default LLMResponse is Empty
    fn default() -> Self {
        LLMResponse::Empty
    }
}

pub fn device(cpu: bool) -> CandleResult<Device> {
    if cpu {
        Ok(Device::Cpu)
    } else {
        let device = Device::cuda_if_available(0)?;
        if !device.is_cuda() {
            println!("Running on CPU, to run on GPU, build this example with `--features cuda`");
        }
        Ok(device)
    }
}
