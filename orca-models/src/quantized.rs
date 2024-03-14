//! Wip for quantized models.
// #![allow(dead_code)]
// #![allow(unused_variables)]
// #![allow(unused_imports)]

use candle::quantized::{ggml_file, gguf_file};
use candle::Device;
use candle_transformers::models::quantized_llama::ModelWeights;

use crate::utils::text_generation::{Model, TextGeneration};

pub struct Config {
    /// The temperature used to generate samples, use 0 for greedy sampling.
    pub temperature: f64,

    /// Nucleus sampling probability cutoff.
    pub top_p: Option<f64>,

    /// The seed to use when generating random samples.
    pub seed: u64,

    /// Penalty to be applied for repeating tokens, 1. means no penalty.
    pub repeat_penalty: f32,

    /// The context size to consider for the repeat penalty.
    pub repeat_last_n: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            temperature: 1.0,
            top_p: None,
            seed: 42,
            repeat_penalty: 1.0,
            repeat_last_n: 1,
        }
    }
}

pub struct Quantized {
    /// The model weights.
    model: ModelWeights,

    /// The tokenizer config.
    tokenizer: tokenizers::Tokenizer,

    /// The temperature used to generate samples, use 0 for greedy sampling.
    temperature: f64,

    /// Nucleus sampling probability cutoff.
    top_p: Option<f64>,

    /// The seed to use when generating random samples.
    seed: u64,

    /// Penalty to be applied for repeating tokens, 1. means no penalty.
    repeat_penalty: f32,

    /// The context size to consider for the repeat penalty.
    repeat_last_n: usize,
}

impl Quantized {
    pub fn from_gguf_stream(model: Vec<u8>, tokenizer: Vec<u8>, config: Config) -> anyhow::Result<Self> {
        let mut model_reader = std::io::Cursor::new(model);
        let model_content = gguf_file::Content::read(&mut model_reader)?;
        let model = ModelWeights::from_gguf(model_content, &mut model_reader)?;
        let tokenizer = tokenizers::Tokenizer::from_bytes(tokenizer).map_err(|m| anyhow::anyhow!(m))?;
        Ok(Self {
            model,
            tokenizer,
            temperature: config.temperature,
            top_p: config.top_p,
            seed: config.seed,
            repeat_penalty: config.repeat_penalty,
            repeat_last_n: config.repeat_last_n,
        })
    }

    pub fn from_ggml_stream(model: Vec<u8>, tokenizer: Vec<u8>, config: Config) -> anyhow::Result<Self> {
        let mut model_reader = std::io::Cursor::new(model);
        let model_content = ggml_file::Content::read(&mut model_reader)?;
        let model = ModelWeights::from_ggml(model_content, 1)?;
        let tokenizer = tokenizers::Tokenizer::from_bytes(tokenizer).map_err(|m| anyhow::anyhow!(m))?;
        Ok(Self {
            model,
            tokenizer,
            temperature: config.temperature,
            top_p: config.top_p,
            seed: config.seed,
            repeat_penalty: config.repeat_penalty,
            repeat_last_n: config.repeat_last_n,
        })
    }

    pub fn generate<W>(&self, prompt: &str, sample_len: usize, output: &mut W) -> anyhow::Result<()>
    where
        W: std::io::Write,
    {
        let mut generator = TextGeneration::new(
            Model::Quantized(self.model.clone()),
            self.tokenizer.clone(),
            self.seed,
            Some(self.temperature),
            self.top_p,
            self.repeat_penalty,
            self.repeat_last_n,
            &candle::Device::Cpu,
        );
        generator.run(prompt, sample_len, output)?;
        Ok(())
    }
}
