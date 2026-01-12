use async_openai::{
    types::{ChatCompletionRequestMessage, CreateChatCompletionRequestArgs},
    Client,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ModelProvider {
    OpenAI,
    Gemini,
    Ollama,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelConfig {
    pub provider: ModelProvider,
    pub model: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
}

pub struct LLMClient {
    pub openai: Option<Client<async_openai::config::OpenAIConfig>>,
    pub gemini_key: Option<String>,
    pub ollama_url: String,
}

impl LLMClient {
    pub fn new() -> Self {
        Self {
            openai: None,
            gemini_key: None,
            ollama_url: "http://localhost:11434/v1".to_string(),
        }
    }

    pub fn set_openai_key(&mut self, key: String) {
        let config = async_openai::config::OpenAIConfig::new().with_api_key(key);
        self.openai = Some(Client::with_config(config));
    }

    pub async fn chat_completion(
        &self,
        config: &ModelConfig,
        messages: Vec<ChatCompletionRequestMessage>,
    ) -> Result<String, String> {
        match config.provider {
            ModelProvider::OpenAI => {
                if let Some(client) = &self.openai {
                    let request = CreateChatCompletionRequestArgs::default()
                        .model(&config.model)
                        .messages(messages)
                        .build()
                        .map_err(|e| e.to_string())?;

                    let response = client
                        .chat()
                        .create(request)
                        .await
                        .map_err(|e| e.to_string())?;
                    Ok(response.choices[0]
                        .message
                        .content
                        .clone()
                        .unwrap_or_default())
                } else {
                    Err("OpenAI client not initialized".to_string())
                }
            }
            ModelProvider::Ollama => {
                let base_url = config.base_url.clone().unwrap_or(self.ollama_url.clone());
                let config_ollama = async_openai::config::OpenAIConfig::new()
                    .with_api_key("ollama")
                    .with_api_base(base_url);
                let client = Client::with_config(config_ollama);

                let request = CreateChatCompletionRequestArgs::default()
                    .model(&config.model)
                    .messages(messages)
                    .build()
                    .map_err(|e| e.to_string())?;

                let response = client
                    .chat()
                    .create(request)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(response.choices[0]
                    .message
                    .content
                    .clone()
                    .unwrap_or_default())
            }
            ModelProvider::Gemini => Err("Gemini provider not fully implemented yet".to_string()),
        }
    }
}
