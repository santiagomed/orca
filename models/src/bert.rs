// use super::console_log;
use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config};
use tokenizers::{PaddingParams, Tokenizer};

pub struct Bert {
    bert: BertModel,
    tokenizer: Tokenizer,
}

impl Bert {
    pub fn from_stream(weights: Vec<u8>, tokenizer: Vec<u8>, config: Vec<u8>) -> anyhow::Result<Self> {
        let device = &Device::Cpu;
        println!("before varbuilder");
        let vb = VarBuilder::from_buffered_safetensors(weights, DType::F64, device)?;
        println!("after varbuilder");
        let config: Config = serde_json::from_slice(&config)?;
        let tokenizer = Tokenizer::from_bytes(&tokenizer).map_err(|m| anyhow::anyhow!(m))?;
        let bert = BertModel::load(vb, &config)?;

        Ok(Self { bert, tokenizer })
    }

    // #[cfg(feature = "async")]
    pub async fn from_api(model_id: Option<String>, revision: Option<String>) -> anyhow::Result<Self> {
        let device = &Device::Cpu;
        let default_model = "sentence-transformers/all-MiniLM-L6-v2".to_string();
        let default_revision = "refs/pr/21".to_string();
        let (model_id, revision) = match (model_id.to_owned(), revision.to_owned()) {
            (Some(model_id), Some(revision)) => (model_id, revision),
            (Some(model_id), None) => (model_id, "main".to_string()),
            (None, Some(revision)) => (default_model, revision),
            (None, None) => (default_model, default_revision),
        };

        let repo = hf_hub::Repo::with_revision(model_id, hf_hub::RepoType::Model, revision);
        let api = hf_hub::api::tokio::Api::new()?;
        let api = api.repo(repo);
        let config_filename = api.get("config.json").await?;
        let tokenizer_filename = api.get("tokenizer.json").await?;
        let weights_filename = api.get("model.safetensors").await?;

        let config = std::fs::read_to_string(config_filename)?;
        let config: Config = serde_json::from_str(&config)?;
        let tokenizer = Tokenizer::from_file(tokenizer_filename).map_err(anyhow::Error::msg)?;

        let vb = VarBuilder::from_buffered_safetensors(std::fs::read(weights_filename)?, DType::F64, device)?;
        let model = BertModel::load(vb, &config)?;
        Ok(Self { bert: model, tokenizer })
    }

    pub fn get_embeddings(&mut self, sentences: &[String], normalize_embedding: bool) -> anyhow::Result<Embeddings> {
        let input: Params = Params {
            sentences: sentences.to_vec(),
            normalize_embeddings: normalize_embedding,
        };
        let sentences = input.sentences;
        let normalize_embeddings = input.normalize_embeddings;

        let device = &Device::Cpu;
        if let Some(pp) = self.tokenizer.get_padding_mut() {
            pp.strategy = tokenizers::PaddingStrategy::BatchLongest
        } else {
            let pp = PaddingParams {
                strategy: tokenizers::PaddingStrategy::BatchLongest,
                ..Default::default()
            };
            self.tokenizer.with_padding(Some(pp));
        }
        let tokens = self.tokenizer.encode_batch(sentences.to_vec(), true).map_err(|m| anyhow::anyhow!(m))?;

        let token_ids: Vec<Tensor> = tokens
            .iter()
            .map(|tokens| {
                let tokens = tokens.get_ids().to_vec();
                Tensor::new(tokens.as_slice(), device)
            })
            .collect::<Result<Vec<_>, _>>()?;

        let token_ids = Tensor::stack(&token_ids, 0)?;
        let token_type_ids = token_ids.zeros_like()?;
        let embeddings = self.bert.forward(&token_ids, &token_type_ids)?;
        // Apply some avg-pooling by taking the mean embedding value for all tokens (including padding)
        let (_n_sentence, n_tokens, _hidden_size) = embeddings.dims3()?;
        let embeddings = (embeddings.sum(1)? / (n_tokens as f64))?;
        let embeddings = if normalize_embeddings {
            embeddings.broadcast_div(&embeddings.sqr()?.sum_keepdim(1)?.sqrt()?)?
        } else {
            embeddings
        };
        let embeddings_data = embeddings.to_vec2()?;
        Ok(Embeddings { data: embeddings_data })
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Embeddings {
    data: Vec<Vec<f64>>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Params {
    sentences: Vec<String>,
    normalize_embeddings: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model() {
        println!("before model");
        let weights = std::path::PathBuf::from("weights/bert_rust_model.ot");
        let tokenizer = std::path::PathBuf::from("weights/bert_tokenizer.json");
        let config = std::path::PathBuf::from("weights/bert_config.json");
        let mut model = Bert::from_stream(
            std::fs::read(weights).unwrap(),
            std::fs::read(tokenizer).unwrap(),
            std::fs::read(config).unwrap(),
        )
        .unwrap();
        let sentences = vec!["This is a sentence".to_string(), "This is another sentence".to_string()];
        let embeddings = model.get_embeddings(&sentences, true).unwrap();
        assert_eq!(embeddings.data.len(), 2);
        assert_eq!(embeddings.data[0].len(), 384);
        assert_eq!(embeddings.data[1].len(), 384);
    }

    #[tokio::test]
    async fn test_model_from_api() {
        let mut model = Bert::from_api(None, None).await.unwrap();
        let sentences = vec!["This is a sentence".to_string(), "This is another sentence".to_string()];
        let embeddings = model.get_embeddings(&sentences, true).unwrap();
        assert_eq!(embeddings.data.len(), 2);
        assert_eq!(embeddings.data[0].len(), 384);
        assert_eq!(embeddings.data[1].len(), 384);
    }
}
