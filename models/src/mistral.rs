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
    pub temperature: f64,

    /// Nucleus sampling probability cutoff.
    pub top_p: Option<f64>,

    /// The seed to use when generating random samples.
    pub seed: u64,

    /// Penalty to be applied for repeating tokens, 1. means no penalty.
    pub repeat_penalty: f32,

    /// The context size to consider for the repeat penalty.
    pub repeat_last_n: usize,

    /// The model id to use.
    pub model_id: Option<String>,

    /// The revision to use.
    pub revision: Option<String>,

    /// Whether to use flash attention.
    pub flash_attn: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            temperature: 1.0,
            top_p: None,
            seed: 42,
            repeat_penalty: 1.0,
            repeat_last_n: 1,
            model_id: Some("lmz/candle-mistral".to_string()),
            revision: Some("main".to_string()),
            flash_attn: false,
        }
    }
}

impl Mistral {
    fn tokenizer<P>(tokenizer: P) -> anyhow::Result<tokenizers::Tokenizer>
    where
        P: AsRef<std::path::Path>,
    {
        tokenizers::Tokenizer::from_file(tokenizer).map_err(|m| anyhow::anyhow!(m))
    }

    pub fn from_path<P>(weights: P, tokenizer: P, config: Config) -> anyhow::Result<Mistral>
    where
        P: AsRef<std::path::Path>,
    {
        let cfg = mistral::Config::config_7b_v0_1(config.flash_attn);
        let vb = candle_transformers::quantized_var_builder::VarBuilder::from_gguf(weights)?;
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

    #[cfg(feature = "async")]
    pub async fn from_api(config: Config) -> anyhow::Result<Self> {
        let api = hf_hub::api::tokio::Api::new()?;
        let repo = api.repo(hf_hub::Repo::with_revision(
            config.model_id.unwrap_or_else(|| "lmz/candle-mistral".to_string()),
            hf_hub::RepoType::Model,
            config.revision.unwrap_or_else(|| "main".to_string()),
        ));
        let tokenizer = repo.get("tokenizer.json").await?;
        let model_path = repo.get("model-q4k.gguf").await?;
        let vb = candle_transformers::quantized_var_builder::VarBuilder::from_gguf(model_path)?;
        let model = quantized_mistral::Model::new(&mistral::Config::config_7b_v0_1(config.flash_attn), vb)?;
        let tokenizer = tokenizers::Tokenizer::from_file(tokenizer).map_err(anyhow::Error::msg)?;
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
            self.model.clone(),
            self.tokenizer.clone(),
            self.seed,
            Some(self.temperature),
            self.top_p,
            self.repeat_penalty,
            self.repeat_last_n,
            &candle_core::Device::Cpu,
        );
        generator.run(prompt, sample_len, output)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires weights"]
    fn test_mistral() {
        let weights = std::path::Path::new("../weights/mistral_model-q4k.gguf");
        let tokenizer = std::path::Path::new("../weights/mistral_tokenizer.json");

        let prompt = "The eiffel tower is";
        let mistral = Mistral::from_path(weights, tokenizer, Config::default()).unwrap();
        let mut output = Vec::new();
        mistral.generate(prompt, 1, &mut output).unwrap();
        assert!(output.len() > 0);
    }

    #[cfg(feature = "async")]
    #[ignore = "downloads weights"]
    #[tokio::test]
    async fn test_mistral_from_api() {
        let prompt = "The eiffel tower is";
        let mistral = Mistral::from_api(Config::default()).await.unwrap();
        let mut output = Vec::new();
        mistral.generate(prompt, 1, &mut output).unwrap();
        assert!(output.len() > 0);
    }
}
