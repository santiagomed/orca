use async_openai::config;
use async_openai::types::CreateChatCompletionRequestArgs;
use serde::Serialize;

use crate::models::openai::error::ModelError;
use crate::prompt::{context::Context, prompt::PromptTemplate};

use super::request::RequestMessages;

#[async_trait::async_trait]
pub trait Generate {
    async fn generate_with_context<T: Serialize + std::marker::Sync + std::fmt::Display>(
        &self,
        name: &str,
        context: &Context<T>,
        template: &PromptTemplate,
    ) -> Result<String, ModelError>;

    async fn generate_with_data<K: Serialize + std::marker::Send + std::fmt::Display>(
        &self,
        name: &str,
        data: K,
        template: &PromptTemplate,
    ) -> Result<String, ModelError>;
}

pub struct OpenAIClient<C: config::Config> {
    /// Client member for the OpenAI API. This client is a wrapper around the async-openai crate, with additional functionality to
    /// support LLM orchestration.
    client: async_openai::Client<C>,

    /// ID of the model to use.
    /// See the [model endpoint compatibility](https://platform.openai.com/docs/models/model-endpoint-compatibility) table for details on which models work with the Chat API.
    model: String,

    /// What sampling temperature to use, between 0 and 2. Higher values like 0.8 will make the output more random,
    /// while lower values like 0.2 will make it more focused and deterministic.
    ///
    /// We generally recommend altering this or `top_p` but not both.
    temperature: f32, // min: 0, max: 2, default: 1,

    /// An alternative to sampling with temperature, called nucleus sampling,
    /// where the model considers the results of the tokens with top_p probability mass.
    /// So 0.1 means only the tokens comprising the top 10% probability mass are considered.
    ///
    ///  We generally recommend altering this or `temperature` but not both.
    top_p: f32, // min: 0, max: 1, default: 1

    /// If set, partial message deltas will be sent, like in ChatGPT.
    /// Tokens will be sent as data-only [server-sent events](https://developer.mozilla.org/en-US/docs/Web/API/Server-sent_events/Using_server-sent_events#Event_stream_format) as they become available,
    /// with the stream terminated by a `data: [DONE]` message.[Example Python code](https://github.com/openai/openai-cookbook/blob/main/examples/How_to_stream_completions.ipynb).
    stream: bool,

    /// The maximum number of [tokens](https://platform.openai.com/tokenizer) to generate in the chat completion.
    ///
    /// The total length of input tokens and generated tokens is limited by the model's context length. [Example Python code](https://github.com/openai/openai-cookbook/blob/main/examples/How_to_count_tokens_with_tiktoken.ipynb) for counting tokens.
    max_tokens: u16,
}

impl OpenAIClient<config::OpenAIConfig> {
    /// Create a new OpenAI client
    pub fn new() -> Self {
        Self {
            client: async_openai::Client::new(),
            model: "gpt-3.5-turbo".to_string(),
            temperature: 1.0,
            top_p: 1.0,
            stream: false,
            max_tokens: 1024u16,
        }
    }

    /// Set model to use
    /// e.g. "davinci", "gpt-3.5-turbo"
    pub fn with_model(mut self, model: &str) -> Self {
        self.model = model.to_string();
        self
    }

    /// What sampling temperature to use, between 0 and 2. Higher values like 0.8 will make the output more random,
    /// while lower values like 0.2 will make it more focused and deterministic.
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature;
        self
    }

    /// An alternative to sampling with temperature, called nucleus sampling,
    /// where the model considers the results of the tokens with top_p probability mass.
    pub fn with_top_p(mut self, top_p: f32) -> Self {
        self.top_p = top_p;
        self
    }

    /// If set, partial message deltas will be sent, like in ChatGPT.
    pub fn with_stream(mut self, stream: bool) -> Self {
        self.stream = stream;
        self
    }

    /// The maximum number of [tokens](https://platform.openai.com/tokenizer) to generate in the chat completion.
    pub fn with_max_tokens(mut self, max_tokens: u16) -> Self {
        self.max_tokens = max_tokens;
        self
    }
}

#[async_trait::async_trait]
impl Generate for OpenAIClient<config::OpenAIConfig> {
    async fn generate_with_context<T: Serialize + std::marker::Sync + std::fmt::Display>(
        &self,
        name: &str,
        context: &Context<T>,
        template: &PromptTemplate,
    ) -> Result<String, ModelError> {
        let prompt = template.render_context(name, context)?;

        let request = CreateChatCompletionRequestArgs::default()
            .model(self.model.clone())
            .messages(RequestMessages::from(prompt))
            .build()?;

        match self.client.chat().create(request).await {
            Ok(response) => Ok(response.choices[0].to_owned().message.content.unwrap()),
            Err(err) => Err(ModelError::OpenAIError(err)),
        }
    }

    async fn generate_with_data<K: Serialize + std::marker::Send + std::fmt::Display>(
        &self,
        name: &str,
        data: K,
        template: &PromptTemplate,
    ) -> Result<String, ModelError> {
        let prompt = template.render_data(name, data)?;

        let request = CreateChatCompletionRequestArgs::default()
            .model(self.model.clone())
            .messages(RequestMessages::from(prompt))
            .build()?;

        match self.client.chat().create(request).await {
            Ok(response) => Ok(response.choices[0].to_owned().message.content.unwrap()),
            Err(err) => Err(ModelError::OpenAIError(err)),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_generate() {
        let client = OpenAIClient::new();
        let mut context = Context::new();
        context.set("country1", "France");
        context.set("country2", "Germany");
        let template = PromptTemplate::new().from_chat(
            "chat",
            vec![
                ("user", "What is the capital of {{country1}}"),
                ("ai", "Paris"),
                ("user", "What is the capital of {{country2}}"),
            ],
        );
        let response = client
            .generate_with_context("chat", &context, &template)
            .await
            .unwrap();
        // contains "Paris" or "paris"
        assert!(response.to_lowercase().contains("berlin"));
    }
}
