#[cfg(feature = "mkl")]
extern crate intel_mkl_src;

#[cfg(feature = "accelerate")]
extern crate accelerate_src;

use tokio::sync::RwLock;

use tokenizers::Tokenizer;

use candle_core::quantized::{ggml_file, gguf_file};
use candle_core::{Device, Tensor};
use candle_transformers::generation::LogitsProcessor;

use anyhow::Result;
use candle_transformers::models::quantized_llama as model;
use model::ModelWeights;

use crate::prompt::chat::{ChatPrompt, Role};

use crate::prompt::Prompt;

use super::{LLMResponse, LLM};

#[derive(Clone, Debug, Copy)]
pub enum Model {
    L7b,
    L13b,
    L70b,
    L7bChat,
    L13bChat,
    L70bChat,
    L7bCode,
    L13bCode,
    L34bCode,
    Mistral7b,
    Mistral7bInstruct,
}

impl Model {
    fn is_mistral(&self) -> bool {
        match self {
            Self::L7b
            | Self::L13b
            | Self::L70b
            | Self::L7bChat
            | Self::L13bChat
            | Self::L70bChat
            | Self::L7bCode
            | Self::L13bCode
            | Self::L34bCode => false,
            Self::Mistral7b | Self::Mistral7bInstruct => true,
        }
    }
}

pub struct Quantized {
    /// The loaded model weights
    model: Option<RwLock<ModelWeights>>,

    /// The path to read the model from.
    model_path: Option<std::path::PathBuf>,

    /// The length of the sample to generate (in tokens).
    sample_len: usize,

    /// The tokenizer config in json format.
    tokenizer: Option<String>,

    /// The temperature used to generate samples, use 0 for greedy sampling.
    temperature: f64,

    /// Nucleus sampling probability cutoff.
    top_p: Option<f64>,

    /// The seed to use when generating random samples.
    seed: u64,

    /// Enable tracing (generates a trace-timestamp.json file).
    tracing: bool,

    /// Display the token for the specified prompt.
    verbose_prompt: bool,

    /// Penalty to be applied for repeating tokens, 1. means no penalty.
    repeat_penalty: f32,

    /// The context size to consider for the repeat penalty.
    repeat_last_n: usize,

    /// The model size to use.
    which: Model,

    /// Group-Query Attention, use 8 for the 70B version of LLaMAv2.
    gqa: Option<usize>,
    //// Use to give context to the prompt for a chat interaction.
    // chat_context: Option<String>,
}

impl Quantized {
    pub fn new() -> Self {
        Self {
            model: None,
            model_path: None,
            sample_len: 99,
            tokenizer: None,
            temperature: 1.,
            top_p: None,
            seed: 42,
            tracing: false,
            verbose_prompt: false,
            repeat_penalty: 1.,
            repeat_last_n: 1,
            which: Model::L7b,
            gqa: None,
            // chat_context: None,
        }
    }

    pub fn with_sample_len(mut self, sample_len: usize) -> Self {
        self.sample_len = sample_len;
        self
    }

    fn tokenizer(&self) -> anyhow::Result<Tokenizer> {
        let tokenizer_path = match &self.tokenizer {
            Some(config) => std::path::PathBuf::from(config),
            None => {
                let api = hf_hub::api::sync::Api::new()?;
                let repo = if self.which.is_mistral() {
                    "mistralai/Mistral-7B-v0.1"
                } else {
                    "hf-internal-testing/llama-tokenizer"
                };
                let api = api.model(repo.to_string());
                api.get("tokenizer.json")?
            }
        };
        Tokenizer::from_file(tokenizer_path).map_err(anyhow::Error::msg)
    }

    pub async fn load_model(mut self, model: Model) -> anyhow::Result<Self> {
        let (repo, filename) = match model {
            Model::L7b => ("TheBloke/Llama-2-7B-GGML", "llama-2-7b.ggmlv3.q4_0.bin"),
            Model::L13b => ("TheBloke/Llama-2-13B-GGML", "llama-2-13b.ggmlv3.q4_0.bin"),
            Model::L70b => ("TheBloke/Llama-2-70B-GGML", "llama-2-70b.ggmlv3.q4_0.bin"),
            Model::L7bChat => ("TheBloke/Llama-2-7B-Chat-GGML", "llama-2-7b-chat.ggmlv3.q4_0.bin"),
            Model::L13bChat => ("TheBloke/Llama-2-13B-Chat-GGML", "llama-2-13b-chat.ggmlv3.q4_0.bin"),
            Model::L70bChat => ("TheBloke/Llama-2-70B-Chat-GGML", "llama-2-70b-chat.ggmlv3.q4_0.bin"),
            Model::L7bCode => ("TheBloke/CodeLlama-7B-GGUF", "codellama-7b.Q8_0.gguf"),
            Model::L13bCode => ("TheBloke/CodeLlama-13B-GGUF", "codellama-13b.Q8_0.gguf"),
            Model::L34bCode => ("TheBloke/CodeLlama-34B-GGUF", "codellama-34b.Q8_0.gguf"),
            Model::Mistral7b => ("TheBloke/Mistral-7B-v0.1-GGUF", "mistral-7b-v0.1.Q4_K_S.gguf"),
            Model::Mistral7bInstruct => (
                "TheBloke/Mistral-7B-Instruct-v0.1-GGUF",
                "mistral-7b-instruct-v0.1.Q4_K_S.gguf",
            ),
        };
        let api = hf_hub::api::tokio::Api::new()?;
        self.model_path = Some(api.model(repo.to_string()).get(filename).await?);
        Ok(self)
    }

    pub fn load_model_from_path(mut self, model_path: &str) -> anyhow::Result<Self> {
        let model_path = std::path::PathBuf::from(model_path);
        if !model_path.exists() {
            return Err(anyhow::Error::msg(format!(
                "model file not found: {}",
                model_path.display()
            )));
        }
        self.model_path = Some(model_path);
        Ok(self)
    }

    pub fn build_model(mut self) -> Result<Self> {
        if self.model_path.is_none() {
            return Err(anyhow::Error::msg("model path not set"));
        }
        let model_path = self.model_path.as_ref().unwrap();
        let mut file = std::fs::File::open(&model_path)?;
        let start = std::time::Instant::now();

        self.model = match model_path.extension().and_then(|v| v.to_str()) {
            Some("gguf") => {
                let model = gguf_file::Content::read(&mut file)?;
                let mut total_size_in_bytes = 0;
                for (_, tensor) in model.tensor_infos.iter() {
                    let elem_count = tensor.shape.elem_count();
                    total_size_in_bytes += elem_count * tensor.ggml_dtype.type_size() / tensor.ggml_dtype.blck_size();
                }
                log::info!(
                    "loaded {:?} tensors ({}) in {:.2}s",
                    model.tensor_infos.len(),
                    &format_size(total_size_in_bytes),
                    start.elapsed().as_secs_f32(),
                );
                Some(RwLock::new(ModelWeights::from_gguf(model, &mut file)?))
            }
            Some("ggml" | "bin") | Some(_) | None => {
                let model = ggml_file::Content::read(&mut file)?;
                let mut total_size_in_bytes = 0;
                for (_, tensor) in model.tensors.iter() {
                    let elem_count = tensor.shape().elem_count();
                    total_size_in_bytes += elem_count * tensor.dtype().type_size() / tensor.dtype().blck_size();
                }
                log::info!(
                    "loaded {:?} tensors ({}) in {:.2}s",
                    model.tensors.len(),
                    &format_size(total_size_in_bytes),
                    start.elapsed().as_secs_f32(),
                );
                log::info!("params: {:?}", model.hparams);
                let default_gqa = match self.which {
                    Model::L7b
                    | Model::L13b
                    | Model::L7bChat
                    | Model::L13bChat
                    | Model::L7bCode
                    | Model::L13bCode
                    | Model::L34bCode => 1,
                    Model::Mistral7b | Model::Mistral7bInstruct | Model::L70b | Model::L70bChat => 8,
                };
                Some(RwLock::new(ModelWeights::from_ggml(
                    model,
                    self.gqa.unwrap_or(default_gqa),
                )?))
            }
        };
        log::info!("model built");
        Ok(self)
    }

    fn format_chat_prompt(&self, chat_prompt: ChatPrompt) -> String {
        let mut prompt = String::new();
        for message in chat_prompt {
            if message.role == Role::System {
                prompt.push_str(message.content.as_str());
            } else {
                prompt.push_str(format!("{}: {}", message.role, message.content).as_str());
            }
        }
        prompt
    }
}

fn get_token(next_token: u32, tokenizer: &Tokenizer, result: &mut String) {
    // Extracting the last token as a string is complicated, here we just apply some simple
    // heuristics as it seems to work well enough for this example. See the following for more
    // details:
    // https://github.com/huggingface/tokenizers/issues/1141#issuecomment-1562644141
    if let Some(text) = tokenizer.id_to_token(next_token) {
        let text = text.replace('▁', " ");
        let ascii = text
            .strip_prefix("<0x")
            .and_then(|t| t.strip_suffix('>'))
            .and_then(|t| u8::from_str_radix(t, 16).ok());

        match ascii {
            None => result.push_str(&text),
            Some(ascii) => {
                if let Some(chr) = char::from_u32(ascii as u32) {
                    if chr.is_ascii() {
                        result.push(chr);
                    }
                }
            }
        }
    }
}

fn format_size(size_in_bytes: usize) -> String {
    if size_in_bytes < 1_000 {
        format!("{}B", size_in_bytes)
    } else if size_in_bytes < 1_000_000 {
        format!("{:.2}KB", size_in_bytes as f64 / 1e3)
    } else if size_in_bytes < 1_000_000_000 {
        format!("{:.2}MB", size_in_bytes as f64 / 1e6)
    } else {
        format!("{:.2}GB", size_in_bytes as f64 / 1e9)
    }
}

#[async_trait::async_trait]
impl LLM for Quantized {
    async fn generate(&self, prompt: Box<dyn Prompt>) -> Result<LLMResponse> {
        use tracing_chrome::ChromeLayerBuilder;
        use tracing_subscriber::prelude::*;

        let temperature = if self.temperature == 0. {
            None
        } else {
            Some(self.temperature)
        };
        let _guard = if self.tracing {
            let (chrome_layer, guard) = ChromeLayerBuilder::new().build();
            tracing_subscriber::registry().with(chrome_layer).init();
            Some(guard)
        } else {
            None
        };

        let tokenizer = self.tokenizer()?;
        let prompt = if prompt.to_chat().is_err() {
            prompt.to_string()?
        } else {
            let chat_prompt = prompt.to_chat()?;
            let prompt = self.format_chat_prompt(chat_prompt);
            if self.verbose_prompt {
                log::info!("prompt:\n{}", &prompt);
            }
            prompt
        };
        let mut result = String::new();

        log::info!("{}", &prompt);
        let tokens = tokenizer.encode(prompt, true).map_err(anyhow::Error::msg)?;
        if self.verbose_prompt {
            for (token, id) in tokens.get_tokens().iter().zip(tokens.get_ids().iter()) {
                let token = token.replace('▁', " ").replace("<0x0A>", "\n");
                log::info!("{id:7} -> '{token}'");
            }
        }

        let prompt_tokens = tokens.get_ids().to_vec();
        let to_sample = self.sample_len.saturating_sub(1);
        let prompt_tokens = if prompt_tokens.len() + to_sample > model::MAX_SEQ_LEN - 10 {
            let to_remove = prompt_tokens.len() + to_sample + 10 - model::MAX_SEQ_LEN;
            prompt_tokens[prompt_tokens.len().saturating_sub(to_remove)..].to_vec()
        } else {
            prompt_tokens
        };
        let mut all_tokens = vec![];
        let mut logits_processor = LogitsProcessor::new(self.seed, temperature, self.top_p);

        let start_prompt_processing = std::time::Instant::now();
        let mut next_token = {
            let input = Tensor::new(prompt_tokens.as_slice(), &Device::Cpu)?.unsqueeze(0)?;
            let logits = self
                .model
                .as_ref()
                .ok_or_else(|| anyhow::Error::msg("model not loaded"))?
                .write()
                .await
                .forward(&input, 0)?;
            let logits = logits.squeeze(0)?;
            logits_processor.sample(&logits)?
        };
        let prompt_dt = start_prompt_processing.elapsed();
        all_tokens.push(next_token);
        get_token(next_token, &tokenizer, &mut result);

        let eos_token = *tokenizer.get_vocab(true).get("</s>").unwrap();

        let start_post_prompt = std::time::Instant::now();
        for index in 0..to_sample {
            let input = Tensor::new(&[next_token], &Device::Cpu)?.unsqueeze(0)?;
            let logits = self
                .model
                .as_ref()
                .ok_or_else(|| anyhow::Error::msg("model not loaded"))?
                .write()
                .await
                .forward(&input, prompt_tokens.len() + index)?;
            let logits = logits.squeeze(0)?;
            let logits = if self.repeat_penalty == 1. {
                logits
            } else {
                let start_at = all_tokens.len().saturating_sub(self.repeat_last_n);
                candle_transformers::utils::apply_repeat_penalty(&logits, self.repeat_penalty, &all_tokens[start_at..])?
            };
            next_token = logits_processor.sample(&logits)?;
            all_tokens.push(next_token);
            get_token(next_token, &tokenizer, &mut result);
            if next_token == eos_token {
                break;
            };
        }
        let dt = start_post_prompt.elapsed();
        log::info!(
            "\n\n{:4} prompt tokens processed: {:.2} token/s",
            prompt_tokens.len(),
            prompt_tokens.len() as f64 / prompt_dt.as_secs_f64(),
        );
        log::info!(
            "{:4} tokens generated: {:.2} token/s",
            to_sample,
            to_sample as f64 / dt.as_secs_f64(),
        );

        Ok(LLMResponse::Quantized(result))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    #[ignore = "needs a file to load from"]
    async fn test_generate() {
        let model = Quantized::new()
            .with_sample_len(1)
            .load_model_from_path("./mistral-7b-v0.1.Q4_0.gguf")
            .unwrap()
            .build_model()
            .unwrap();
        let response = model.generate(Box::new("I am".to_string())).await.unwrap();
        println!("{:?}", response.to_string());
    }
}
