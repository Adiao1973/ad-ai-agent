use anyhow::{anyhow, Result};
use futures_util::StreamExt;
use std::sync::Arc;
use tokio::sync::Mutex;

use rust_agent_core::api::{ChatMessage, DeepseekClient};
use rust_agent_core::tools::{
    format_tool_result, parse_tool_calls, ToolParameters, ToolResult, ToolsClient,
};

pub struct ChatSession {
    client: DeepseekClient,
    messages: Vec<ChatMessage>,
    verbose: bool,
    tools_client: Option<Arc<Mutex<ToolsClient>>>,
}

impl ChatSession {
    pub fn new(api_key: String, verbose: bool) -> Self {
        Self {
            client: DeepseekClient::new(api_key),
            messages: Vec::new(),
            verbose,
            tools_client: None,
        }
    }

    /// 设置工具客户端
    pub fn set_tools_client(&mut self, client: ToolsClient) {
        self.tools_client = Some(Arc::new(Mutex::new(client)));
    }

    /// 检查是否已连接工具服务
    pub fn has_tools(&self) -> bool {
        self.tools_client.is_some()
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

    pub fn add_system_message(&mut self, content: String) {
        self.messages.push(ChatMessage {
            role: "system".to_string(),
            content,
        });
    }

    /// 获取 AI 响应并处理工具调用（流式输出）
    pub async fn get_response_stream<F>(&self, mut callback: F) -> Result<String>
    where
        F: FnMut(&str),
    {
        let mut stream = self.client.chat_stream(self.messages.clone()).await?;
        let mut full_response = String::new();

        while let Some(chunk) = stream.next().await {
            let content = chunk?;
            if !content.is_empty() {
                callback(&content);
                full_response.push_str(&content);
            }
        }

        // 检查是否包含工具调用
        let tool_calls = parse_tool_calls(&full_response);
        if !tool_calls.is_empty() && self.tools_client.is_some() {
            let mut result_content = full_response.clone();

            // 逐个执行工具调用
            for tool_params in tool_calls {
                let tool_name = tool_params.name.clone();
                callback(&format!("\n执行工具 `{}`...\n", tool_name));

                match self.execute_tool(tool_params).await {
                    Ok(result) => {
                        let result_text = format_tool_result(&tool_name, &result);
                        result_content.push_str("\n\n");
                        result_content.push_str(&result_text);
                        callback("\n\n");
                        callback(&result_text);
                    }
                    Err(e) => {
                        let error_text = format!("工具 `{}` 执行失败: {}", tool_name, e);
                        result_content.push_str("\n\n");
                        result_content.push_str(&error_text);
                        callback("\n\n");
                        callback(&error_text);
                    }
                }
            }

            Ok(result_content)
        } else {
            Ok(full_response)
        }
    }

    /// 执行工具调用
    async fn execute_tool(&self, params: ToolParameters) -> Result<ToolResult> {
        if let Some(tools_client) = &self.tools_client {
            let mut client = tools_client.lock().await;
            client.execute_tool(params).await
        } else {
            Err(anyhow!("工具客户端未初始化"))
        }
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
