use crate::utils::text_generation::TextGeneration;
use candle_transformers::models::mistral;
use candle_transformers::models::quantized_mistral;

pub struct Mistral {
    /// The model to use.
    model: quantized_mistral::Model,

    /// The tokenizer config in json format.
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

pub struct Config {
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

    flash_attn: bool,
}

impl Mistral {
    fn tokenizer(tokenizer: Vec<u8>) -> anyhow::Result<tokenizers::Tokenizer> {
        tokenizers::Tokenizer::from_bytes(tokenizer).map_err(|m| anyhow::anyhow!(m))
    }

    pub fn from_stream(weights: Vec<u8>, tokenizer: Vec<u8>, config: Config) -> anyhow::Result<Mistral> {
        let cfg = mistral::Config::config_7b_v0_1(config.flash_attn);
        let vb = candle_transformers::quantized_var_builder::VarBuilder::from_gguf_buffer(&weights)?;
        let model = quantized_mistral::Model::new(&cfg, vb)?;
        let tokenizer = Mistral::tokenizer(tokenizer)?;
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

    async fn tokenizer_api() -> anyhow::Result<tokenizers::Tokenizer> {
        let api = hf_hub::api::tokio::Api::new()?;
        let repo = "mistralai/Mistral-7B-v0.1";
        let api = api.model(repo.to_string());
        let tokenizer_file = api.get("tokenizer.json").await?;
        tokenizers::Tokenizer::from_file(tokenizer_file).map_err(|m| anyhow::anyhow!(m))
    }

    pub async fn from_api(config: Config, instruct: bool) -> anyhow::Result<Self> {
        let (repo, filename) = if instruct {
            (
                "TheBloke/Mistral-7B-Instruct-v0.1-GGUF",
                "mistral-7b-instruct-v0.1.Q4_K_S.gguf",
            )
        } else {
            ("TheBloke/Mistral-7B-v0.1-GGUF", "mistral-7b-v0.1.Q4_K_S.gguf")
        };
        let api = hf_hub::api::tokio::Api::new()?;
        let model_path = api.model(repo.to_string()).get(filename).await?;
        let vb = candle_transformers::quantized_var_builder::VarBuilder::from_gguf(model_path)?;
        let model = quantized_mistral::Model::new(&mistral::Config::config_7b_v0_1(config.flash_attn), vb)?;
        let tokenizer = Mistral::tokenizer_api().await?;
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

    pub fn generate(&self, prompt: &str, sample_len: usize) -> anyhow::Result<()> {
        let mut generator = TextGeneration::new(
            self.model.clone(),
            self.tokenizer.clone(),
            self.seed,
            Some(self.temperature),
            self.top_p,
            self.repeat_penalty,
            self.repeat_last_n,
            &candle_core::Device::Cpu,
        );
        let mut output = std::io::stdout();
        generator.run(prompt, sample_len, &mut output)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mistral() {
        let prompt = "[INST]The quick brown fox jumps over the lazy dog.[/INST]";
        let mistral = Mistral::from_stream(
            include_bytes!("../../weights/mistral-7b-instruct-v0.1.Q4_K_S.gguf").to_vec(),
            include_bytes!("../../weights/mistral_tokenizer.json").to_vec(),
            Config {
                temperature: 0.7,
                top_p: None,
                seed: 42,
                repeat_penalty: 1.0,
                repeat_last_n: 1,
                flash_attn: false,
            },
        )
        .unwrap();
        mistral.generate(prompt, 10).unwrap();
    }
}
