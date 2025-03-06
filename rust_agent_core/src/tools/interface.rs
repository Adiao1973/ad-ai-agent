use anyhow::Result;
use serde::{Deserialize, Serialize};

/// 工具调用的参数
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolParameters {
    pub name: String,
    pub args: serde_json::Value,
}

/// 工具调用的结果
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub data: serde_json::Value,
    pub error: Option<String>,
}

/// 工具特征定义
#[async_trait::async_trait]
pub trait Tool: Send + Sync {
    /// 获取工具名称
    fn name(&self) -> &str;

    /// 获取工具描述
    fn description(&self) -> &str;

    /// 执行工具
    async fn execute(&self, params: ToolParameters) -> Result<ToolResult>;
}
