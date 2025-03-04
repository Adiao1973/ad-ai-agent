use anyhow::Result;

use crate::api::{ChatMessage, DeepseekClient};

pub struct ChatSession {
    client: DeepseekClient,
    messages: Vec<ChatMessage>,
    verbose: bool,
}

impl ChatSession {
    pub fn new(api_key: String, verbose: bool) -> Self {
        Self {
            client: DeepseekClient::new(api_key),
            messages: Vec::new(),
            verbose,
        }
    }

    pub fn add_user_message(&mut self, content: String) {
        self.messages.push(ChatMessage {
            role: "user".to_string(),
            content,
        });
    }

    pub fn add_assistant_message(&mut self, content: String) {
        self.messages.push(ChatMessage {
            role: "assistant".to_string(),
            content,
        });
    }

    pub async fn get_response(&self) -> Result<String> {
        self.client.chat(self.messages.clone()).await
    }

    pub fn remove_last_message(&mut self) {
        self.messages.pop();
    }

    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    pub fn is_verbose(&self) -> bool {
        self.verbose
    }
}
