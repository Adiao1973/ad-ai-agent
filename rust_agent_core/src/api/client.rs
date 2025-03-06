use anyhow::Result;

use super::types::{ChatMessage, ChatRequest, ChatResponse};

pub struct DeepseekClient {
    client: reqwest::Client,
    api_key: String,
}

impl DeepseekClient {
    pub fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
        }
    }

    pub async fn chat(&self, messages: Vec<ChatMessage>) -> Result<String> {
        let request = ChatRequest {
            model: "deepseek-chat".to_string(),
            messages,
            temperature: 0.7,
        };

        let response = self
            .client
            .post("https://api.deepseek.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?
            .json::<ChatResponse>()
            .await?;

        Ok(response.choices[0].message.content.clone())
    }
}
