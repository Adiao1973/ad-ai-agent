use anyhow::{anyhow, Result};
use regex::Regex;
use serde_json::Value;

use crate::tools::interface::{ToolParameters, ToolResult};

/// 工具调用标记
const TOOL_CALL_START: &str = "```tool";
const TOOL_CALL_END: &str = "```";

/// 解析 AI 回复中的工具调用
pub fn parse_tool_calls(ai_message: &str) -> Vec<ToolParameters> {
    let mut tool_calls = Vec::new();

    // 使用正则表达式匹配工具调用块
    let re = Regex::new(r"```tool\s*\n([\s\S]*?)\n```").unwrap();

    for cap in re.captures_iter(ai_message) {
        if let Some(tool_content) = cap.get(1) {
            if let Ok(params) = parse_tool_content(tool_content.as_str()) {
                tool_calls.push(params);
            }
        }
    }

    tool_calls
}

/// 解析工具调用内容
fn parse_tool_content(content: &str) -> Result<ToolParameters> {
    // 尝试解析 JSON 格式
    if let Ok(json_value) = serde_json::from_str::<Value>(content) {
        if let Some(name) = json_value.get("name").and_then(|n| n.as_str()) {
            let args = json_value
                .get("args")
                .cloned()
                .unwrap_or(Value::Object(serde_json::Map::new()));

            return Ok(ToolParameters {
                name: name.to_string(),
                args,
            });
        }
    }

    // 尝试解析简单格式 (name: args)
    let parts: Vec<&str> = content.splitn(2, ':').collect();
    if parts.len() == 2 {
        let name = parts[0].trim();
        let args_str = parts[1].trim();

        // 尝试将参数解析为 JSON
        let args = serde_json::from_str(args_str).unwrap_or(Value::String(args_str.to_string()));

        return Ok(ToolParameters {
            name: name.to_string(),
            args,
        });
    }

    Err(anyhow!("无法解析工具调用内容"))
}

/// 格式化工具调用结果
pub fn format_tool_result(tool_name: &str, result: &ToolResult) -> String {
    let mut output = format!("工具 `{}` 执行", tool_name);

    if result.success {
        output.push_str("成功：\n\n");

        // 格式化结果数据
        let formatted_data = match &result.data {
            Value::String(s) => s.clone(),
            _ => serde_json::to_string_pretty(&result.data).unwrap_or_default(),
        };

        output.push_str(&formatted_data);
    } else {
        output.push_str("失败：\n\n");
        if let Some(error) = &result.error {
            output.push_str(error);
        } else {
            output.push_str("未知错误");
        }
    }

    output
}
