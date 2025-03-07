use anyhow::Result;
use futures::Stream;
use tokio_stream::StreamExt;

use super::types::{ChatMessage, ChatRequest, ChatResponse, ChatStreamResponse};

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
            stream: false,
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

    pub async fn chat_stream(
        &self,
        messages: Vec<ChatMessage>,
    ) -> Result<impl Stream<Item = Result<String>>> {
        let request = ChatRequest {
            model: "deepseek-chat".to_string(),
            messages,
            temperature: 0.7,
            stream: true,
        };

        let response = self
            .client
            .post("https://api.deepseek.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        let stream = response.bytes_stream().map(|chunk| {
            chunk.map_err(|e| anyhow::anyhow!(e)).and_then(|bytes| {
                if bytes.is_empty() {
                    return Ok(String::new());
                }

                let text = String::from_utf8(bytes.to_vec())?;
                let mut responses = Vec::new();

                for line in text.lines() {
                    let line = line.trim();
                    if line.starts_with("data: ") {
                        let json_str = line.trim_start_matches("data: ");
                        if json_str == "[DONE]" {
                            continue;
                        }
                        if let Ok(stream_response) =
                            serde_json::from_str::<ChatStreamResponse>(json_str)
                        {
                            if let Some(choice) = stream_response.choices.first() {
                                if !choice.delta.content.is_empty() {
                                    responses.push(choice.delta.content.clone());
                                }
                            }
                        }
                    }
                }

                if responses.is_empty() {
                    Ok(String::new())
                } else {
                    Ok(responses.join(""))
                }
            })
        });

        Ok(stream)
    }
}
