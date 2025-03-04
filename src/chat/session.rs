use anyhow::{anyhow, Result};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::api::{ChatMessage, DeepseekClient};
use crate::tools::{format_tool_result, parse_tool_calls, ToolParameters, ToolResult, ToolsClient};

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

    /// 获取 AI 响应并处理工具调用
    pub async fn get_response(&self) -> Result<String> {
        let response = self.client.chat(self.messages.clone()).await?;

        // 如果没有工具客户端，直接返回响应
        if self.tools_client.is_none() {
            return Ok(response);
        }

        // 解析工具调用
        let tool_calls = parse_tool_calls(&response);
        if tool_calls.is_empty() {
            return Ok(response);
        }

        // 执行工具调用
        let mut result_content = response.clone();
        for tool_params in tool_calls {
            let tool_name = tool_params.name.clone();

            // 执行工具
            match self.execute_tool(tool_params).await {
                Ok(result) => {
                    // 格式化结果并添加到响应中
                    let result_text = format_tool_result(&tool_name, &result);
                    result_content.push_str("\n\n");
                    result_content.push_str(&result_text);
                }
                Err(e) => {
                    // 添加错误信息
                    result_content.push_str("\n\n");
                    result_content.push_str(&format!("工具 `{}` 执行失败: {}", tool_name, e));
                }
            }
        }

        Ok(result_content)
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
