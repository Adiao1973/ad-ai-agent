use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: f32,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ChatResponse {
    pub choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Choice {
    pub message: Message,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Message {
    pub content: String,
}
