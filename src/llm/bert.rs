use std::sync::Arc;

use anyhow::{anyhow, Error as E, Result};
use candle_core::Tensor;
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config, DTYPE};
use hf_hub::{api::sync::Api, Cache, Repo, RepoType};
use tokenizers::Tokenizer;

use super::{LLMResponse, LLM};

#[derive(Clone)]
pub struct Bert {
    /// Run on CPU rather than on GPU.
    cpu: bool,

    /// Run offline (you must have the files already cached)
    offline: bool,

    /// Enable tracing (generates a trace-timestamp.json file).
    tracing: bool,

    /// The model to use, check out available models: https://huggingface.co/models?library=sentence-transformers&sort=trending
    model_id: Option<String>,

    revision: Option<String>,

    /// When set, compute embeddings for this prompt.
    prompt: Option<String>,

    /// The number of times to run the prompt.
    n: usize,

    /// L2 normalization for embeddings.
    normalize_embeddings: bool,
}

impl Default for Bert {
    fn default() -> Self {
        Self {
            cpu: false,
            offline: false,
            tracing: false,
            model_id: None,
            revision: None,
            prompt: None,
            n: 1,
            normalize_embeddings: false,
        }
    }
}

impl AsRef<Self> for Bert {
    fn as_ref(&self) -> &Self {
        self
    }
}

impl Bert {
    pub fn new(prompt: &str) -> Self {
        Self::default()
    }

    pub fn with_cpu(mut self) -> Self {
        self.cpu = true;
        self
    }

    pub fn offline(mut self) -> Self {
        self.offline = true;
        self
    }

    pub fn with_tracing(mut self) -> Self {
        self.tracing = true;
        self
    }

    pub fn with_model_id(mut self, model_id: &str) -> Self {
        self.model_id = Some(model_id.to_string());
        self
    }

    pub fn with_revision(mut self, revision: &str) -> Self {
        self.revision = Some(revision.to_string());
        self
    }

    pub fn with_prompt(mut self, prompt: &str) -> Self {
        self.prompt = Some(prompt.to_string());
        self
    }

    pub fn with_n(mut self, n: usize) -> Self {
        self.n = n;
        self
    }

    pub fn with_normalize_embeddings(mut self) -> Self {
        self.normalize_embeddings = true;
        self
    }

    fn build_model_and_tokenizer(&self) -> Result<(BertModel, Tokenizer)> {
        let device = super::device(self.cpu)?;
        let default_model = "sentence-transformers/all-MiniLM-L6-v2".to_string();
        let default_revision = "refs/pr/21".to_string();
        let (model_id, revision) = match (self.model_id.to_owned(), self.revision.to_owned()) {
            (Some(model_id), Some(revision)) => (model_id, revision),
            (Some(model_id), None) => (model_id, "main".to_string()),
            (None, Some(revision)) => (default_model, revision),
            (None, None) => (default_model, default_revision),
        };

        let repo = Repo::with_revision(model_id, RepoType::Model, revision);
        let (config_filename, tokenizer_filename, weights_filename) = if self.offline {
            let cache = Cache::default().repo(repo);
            (
                cache.get("config.json").ok_or(anyhow!("Missing config file in cache"))?,
                cache.get("tokenizer.json").ok_or(anyhow!("Missing tokenizer file in cache"))?,
                cache.get("model.safetensors").ok_or(anyhow!("Missing weights file in cache"))?,
            )
        } else {
            let api = Api::new()?;
            let api = api.repo(repo);
            (
                api.get("config.json")?,
                api.get("tokenizer.json")?,
                api.get("model.safetensors")?,
            )
        };
        let config = std::fs::read_to_string(config_filename)?;
        let config: Config = serde_json::from_str(&config)?;
        let tokenizer = Tokenizer::from_file(tokenizer_filename).map_err(E::msg)?;

        let vb = unsafe { VarBuilder::from_mmaped_safetensors(&[weights_filename], DTYPE, &device)? };
        let model = BertModel::load(vb, &config)?;
        Ok((model, tokenizer))
    }
}

#[async_trait::async_trait(?Send)]
impl LLM for Bert {
    async fn generate(&self) -> Result<LLMResponse> {
        use tracing_chrome::ChromeLayerBuilder;
        use tracing_subscriber::prelude::*;

        let _guard = if self.tracing {
            println!("tracing...");
            let (chrome_layer, guard) = ChromeLayerBuilder::new().build();
            tracing_subscriber::registry().with(chrome_layer).init();
            Some(guard)
        } else {
            None
        };

        let cloned = self.clone();
        let (model, mut tokenizer) = tokio::task::spawn_blocking(move || cloned.build_model_and_tokenizer()).await??;
        let model = Arc::new(model);
        let device = &model.device;

        let tokenizer = tokenizer.with_padding(None).with_truncation(None).map_err(E::msg)?;
        let tokens = tokenizer.encode(self.prompt.unwrap(), true).map_err(E::msg)?.get_ids().to_vec();
        let token_ids = Tensor::new(&tokens[..], device)?.unsqueeze(0)?;
        let token_type_ids = token_ids.zeros_like()?;
        let mut out_tensors = Vec::<Tensor>::with_capacity(self.n);
        for idx in 0..self.n {
            let start = std::time::Instant::now();
            let model = model.clone();
            let ys = tokio::task::spawn_blocking(move || model.forward(&token_ids, &token_type_ids)).await??;
            out_tensors.push(ys);
            println!("Took {:?}", start.elapsed());
        }
        Ok(LLMResponse::Bert(out_tensors))
    }
}
