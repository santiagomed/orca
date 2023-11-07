use candle::quantized::gguf_file;
use candle_transformers::models::mistral;
use candle_transformers::models::quantized_mistral;

pub struct Mistral {
    /// The model to use.
    model: quantized_mistral::Model,

    /// The tokenizer config in json format.
    tokenizer: tokenizers::Tokenizer,

    /// The length of the sample to generate (in tokens).
    sample_len: usize,

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

    flash_attn: bool,
}

pub struct Config {
    /// The length of the sample to generate (in tokens).
    sample_len: usize,

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

    flash_attn: bool,
}

impl Mistral {
    fn tokenizer(tokenizer: Vec<u8>) -> anyhow::Result<tokenizers::Tokenizer> {
        tokenizers::Tokenizer::from_bytes(tokenizer).map_err(|m| anyhow::anyhow!(m))
    }

    async fn tokenizer_api() -> anyhow::Result<tokenizers::Tokenizer> {
        let api = hf_hub::api::tokio::Api::new()?;
        let repo = "mistralai/Mistral-7B-v0.1";
        let api = api.model(repo.to_string());
        let tokenizer_file = api.get("tokenizer.json").await?;
        tokenizers::Tokenizer::from_file(tokenizer_file).map_err(|m| anyhow::anyhow!(m))
    }

    pub fn from_stream(weights: Vec<u8>, tokenizer: Vec<u8>, config: Config) -> anyhow::Result<Mistral> {
        let cfg = mistral::Config::config_7b_v0_1(config.flash_attn);
        let vb = candle_transformers::quantized_var_builder::VarBuilder::from_gguf_buffer(&weights)?;
        let model = quantized_mistral::Model::new(&cfg, vb)?;
        let tokenizer = Mistral::tokenizer(tokenizer)?;
        Ok(Self {
            model,
            tokenizer,
            sample_len: config.sample_len,
            temperature: config.temperature,
            top_p: config.top_p,
            seed: config.seed,
            tracing: config.tracing,
            verbose_prompt: config.verbose_prompt,
            repeat_penalty: config.repeat_penalty,
            repeat_last_n: config.repeat_last_n,
            flash_attn: config.flash_attn,
        })
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
            sample_len: config.sample_len,
            temperature: config.temperature,
            top_p: config.top_p,
            seed: config.seed,
            tracing: config.tracing,
            verbose_prompt: config.verbose_prompt,
            repeat_penalty: config.repeat_penalty,
            repeat_last_n: config.repeat_last_n,
            flash_attn: config.flash_attn,
        })
    }
}
