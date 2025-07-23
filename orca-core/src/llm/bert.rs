//! This module provides an implementation for Bert in the context of language models.
//! It utilizes the [candle](https://github.com/huggingface/candle) ML framework.
//!
//! This Bert struct allows for various configuration options such as running on a CPU,
//! offline mode, tracing and model selection among others.

use anyhow::{anyhow, Error as E, Result};
use candle_core::Tensor;
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config, DTYPE};
use hf_hub::{api::tokio::Api, Cache, Repo, RepoType};
use rayon::prelude::*;
use std::sync::{Arc, Mutex};
use tokenizers::{PaddingParams, Tokenizer};
use tokio::sync::RwLock;

use crate::prompt::Prompt;

use super::{Embedding, EmbeddingResponse};

pub struct Bert {
    /// Run on CPU rather than on GPU.
    cpu: bool,

    /// Run offline (you must have the files already cached)
    offline: bool,

    /// Enable tracing (generates a trace-timestamp.json file).
    tracing: bool,

    /// The model to use, check out available models: https://huggingface.co/models?library=sentence-transformers&sort=trending
    model_id: Option<String>,

    /// Model weights.
    model: Option<Arc<BertModel>>,

    /// Tokenizer.
    tokenizer: Option<RwLock<Tokenizer>>,

    revision: Option<String>,

    /// L2 normalization for embeddings.
    normalize_embeddings: bool,
}

impl Default for Bert {
    /// Provides default values for `Bert`.
    fn default() -> Self {
        Self {
            cpu: true,
            offline: false,
            tracing: false,
            model_id: None,
            model: None,
            tokenizer: None,
            revision: None,
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
    /// Creates a new `Bert` instance with a specified prompt.
    pub fn new() -> Self {
        Self::default()
    }

    /// Configures the model to run on CPU.
    pub fn with_cpu(mut self) -> Self {
        self.cpu = false;
        self
    }

    /// Configures the model to run offline.
    pub fn offline(mut self) -> Self {
        self.offline = true;
        self
    }

    /// Enables tracing for the model.
    pub fn with_tracing(mut self) -> Self {
        self.tracing = true;
        self
    }

    /// Sets the model ID.
    pub fn with_model_id(mut self, model_id: &str) -> Self {
        self.model_id = Some(model_id.to_string());
        self
    }

    /// Sets the revision for the model.
    pub fn with_revision(mut self, revision: &str) -> Self {
        self.revision = Some(revision.to_string());
        self
    }

    /// Enables L2 normalization for embeddings.
    pub fn with_normalize_embeddings(mut self) -> Self {
        self.normalize_embeddings = true;
        self
    }

    /// Builds the model and tokenizer.
    pub async fn build_model_and_tokenizer(mut self) -> Result<Self> {
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
                api.get("config.json").await?,
                api.get("tokenizer.json").await?,
                api.get("model.safetensors").await?,
            )
        };
        let config = std::fs::read_to_string(config_filename)?;
        let config: Config = serde_json::from_str(&config)?;
        let tokenizer = Tokenizer::from_file(tokenizer_filename).map_err(E::msg)?;

        let vb = unsafe { VarBuilder::from_mmaped_safetensors(&[weights_filename], DTYPE, &device)? };
        let model = BertModel::load(vb, &config)?;
        self.model = Some(Arc::new(model));
        self.tokenizer = Some(RwLock::new(tokenizer));
        Ok(self)
    }
}

#[async_trait::async_trait]
impl Embedding for Bert {
    async fn generate_embedding(&self, prompt: Box<dyn Prompt>) -> Result<EmbeddingResponse> {
        use tracing_chrome::ChromeLayerBuilder;
        use tracing_subscriber::prelude::*;

        if self.model.is_none() || self.tokenizer.is_none() {
            return Err(anyhow!("Model or tokenizer not initialized"));
        }

        let _guard = if self.tracing {
            log::info!("tracing...");
            let (chrome_layer, guard) = ChromeLayerBuilder::new().build();
            tracing_subscriber::registry().with(chrome_layer).init();
            Some(guard)
        } else {
            None
        };

        let model = self.model.as_ref().unwrap().clone();
        let mut tokenizer = self.tokenizer.as_ref().unwrap().write().await;
        let device = &model.device;
        let prompt = prompt.to_string();
        let tokenizer = tokenizer.with_padding(None).with_truncation(None).map_err(E::msg)?;
        let tokens = tokenizer.encode(prompt, true).map_err(E::msg)?.get_ids().to_vec();
        let token_ids = Tensor::new(&tokens[..], device)?.unsqueeze(0)?;
        log::info!("token_ids shape: {:?}", token_ids.shape());
        let token_type_ids = token_ids.zeros_like()?;
        log::info!("running inference {:?}", token_ids.shape());
        let start = std::time::Instant::now();
        // TODO: Validate the use of `attention_mask`
        let embedding = model.forward(&token_ids, &token_type_ids, None)?;
        log::info!("embedding shape: {:?}", embedding.shape());
        log::info!("Embedding took {:?} to generate", start.elapsed());
        Ok(EmbeddingResponse::Bert(embedding))
    }

    async fn generate_embeddings(&self, prompts: Vec<Box<dyn Prompt>>) -> Result<EmbeddingResponse> {
        use tracing_chrome::ChromeLayerBuilder;
        use tracing_subscriber::prelude::*;

        if self.model.is_none() || self.tokenizer.is_none() {
            return Err(anyhow!("Model or tokenizer not initialized"));
        }

        let _guard = if self.tracing {
            log::info!("tracing...");
            let (chrome_layer, guard) = ChromeLayerBuilder::new().build();
            tracing_subscriber::registry().with(chrome_layer).init();
            Some(guard)
        } else {
            None
        };

        let model: Arc<BertModel> = self.model.as_ref().unwrap().clone();
        let mut tokenizer: tokio::sync::RwLockWriteGuard<'_, Tokenizer> =
            self.tokenizer.as_ref().unwrap().write().await;
        let device = &model.device;

        if let Some(pp) = tokenizer.get_padding_mut() {
            pp.strategy = tokenizers::PaddingStrategy::BatchLongest
        } else {
            let pp = PaddingParams {
                strategy: tokenizers::PaddingStrategy::BatchLongest,
                ..Default::default()
            };
            tokenizer.with_padding(Some(pp));
        }

        let tokens = tokenizer
            .encode_batch(prompts.iter().map(|p| p.to_string()).collect::<Vec<_>>(), true)
            .map_err(E::msg)?;
        let token_ids = tokens
            .iter()
            .enumerate()
            .map(|(i, tokens)| {
                let tokens = tokens.get_ids().to_vec();
                let tensor = Tensor::new(tokens.as_slice(), device)?.unsqueeze(0)?;
                Ok((i, tensor))
            })
            .collect::<Result<Vec<_>>>()?;

        let embeddings = vec![Tensor::ones((2, 3), candle_core::DType::F32, device)?; token_ids.len()];
        // Wrap the embeddings vector in an Arc<Mutex<_>> for thread-safe access
        let embeddings_arc = Arc::new(Mutex::new(embeddings));

        // Use rayon to compute embeddings in parallel
        log::info!("Computing embeddings");
        let start = std::time::Instant::now();
        token_ids.par_iter().try_for_each_with(embeddings_arc.clone(), |embeddings_arc, (i, token_ids)| {
            let token_type_ids = token_ids.zeros_like()?;
            // TODO: Validate the use of `attention_mask`
            let embedding = model.forward(token_ids, &token_type_ids, None)?.squeeze(0)?;

            // Lock the mutex and write the embedding to the correct index
            let mut embeddings = embeddings_arc.lock().map_err(|e| anyhow!("Mutex error: {}", e))?;
            embeddings[*i] = embedding;

            Ok::<(), anyhow::Error>(())
        })?;
        log::info!("Done computing embeddings");
        log::info!("Embeddings took {:?} to generate", start.elapsed());

        // Retrieve the final ordered embeddings
        let embeddings_arc = Arc::try_unwrap(embeddings_arc)
            .map_err(|_| anyhow!("Arc unwrap failed"))?
            .into_inner()
            .map_err(|e| anyhow!("Mutex error: {}", e))?;

        let stacked_embeddings = Tensor::stack(&embeddings_arc, 0)?;

        Ok(EmbeddingResponse::Bert(stacked_embeddings))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{prompt, prompts};

    #[tokio::test]
    async fn test_generate() {
        let bert = Bert::new();
        let response =
            bert.build_model_and_tokenizer().await.unwrap().generate_embedding(prompt!("In the heart of the ancient forest, a single leaf fluttered, carrying the secret language of trees. It landed in the palm of a child, whispering tales of centuries past and futures yet to unfold.")).await;
        let response = response.unwrap();
        let vec = response.to_vec().unwrap();
        assert_eq!(vec.len(), 384);
    }

    #[tokio::test]
    async fn test_batch() {
        let bert = Bert::new().build_model_and_tokenizer().await.unwrap();
        let response = bert.generate_embeddings(prompts!("Hello World", "Goodbye World")).await;
        let response = response.unwrap();
        let vec = response.to_vec2().unwrap();
        assert_eq!(vec.len(), 2);
        assert_eq!(vec[0].len(), 384);
        assert_eq!(vec[1].len(), 384);
    }
}
