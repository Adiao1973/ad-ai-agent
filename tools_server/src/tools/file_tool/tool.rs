use anyhow::{anyhow, Result};
use std::path::Path;
use std::time::Instant;
use tracing::{debug, error, info};

use super::converter::FileConverter;
use super::types::{FileDetails, FileOperation, FileToolParams, FileToolResponse};
use async_trait::async_trait;
use rust_agent_core::tools::interface::{Tool, ToolParameters, ToolResult};

pub struct FileTool {
    converter: FileConverter,
}

impl FileTool {
    pub fn new() -> Result<Self> {
        Ok(Self {
            converter: FileConverter::new()?,
        })
    }

    async fn convert_file(&self, params: &FileToolParams) -> Result<FileToolResponse> {
        let input = Path::new(&params.input);
        let output = params
            .output
            .as_ref()
            .map(Path::new)
            .ok_or_else(|| anyhow!("需要指定输出路径"))?;

        let options = params
            .options
            .as_ref()
            .ok_or_else(|| anyhow!("需要指定转换选项"))?;

        if !input.exists() {
            return Err(anyhow!("输入文件不存在"));
        }

        debug!("开始文件转换: {:?} -> {:?}", input, output);
        let start = Instant::now();
        let original_size = input.metadata()?.len();

        self.converter.convert(input, output, options).await?;

        let processed_size = output.metadata()?.len();
        let processing_time = start.elapsed().as_secs_f64();

        info!(
            "文件转换完成: 原始大小={}, 处理后大小={}, 耗时={:.2}s",
            original_size, processed_size, processing_time
        );

        Ok(FileToolResponse {
            success: true,
            message: "转换成功".to_string(),
            output_path: Some(output.to_string_lossy().to_string()),
            details: Some(FileDetails {
                original_size,
                processed_size,
                processing_time,
            }),
        })
    }
}

#[async_trait]
impl Tool for FileTool {
    fn name(&self) -> &str {
        "file_tool"
    }

    fn description(&self) -> &str {
        "文件处理工具，支持文件转换、压缩、解压、重命名和整理等操作"
    }

    async fn execute(&self, params: ToolParameters) -> Result<ToolResult> {
        info!("执行文件处理工具，参数: {:?}", params);

        let params: FileToolParams = match serde_json::from_value(params.args) {
            Ok(p) => p,
            Err(e) => {
                error!("参数解析失败: {}", e);
                return Ok(ToolResult {
                    success: false,
                    data: serde_json::Value::Null,
                    error: Some(e.to_string()),
                });
            }
        };

        let result = match params.operation {
            FileOperation::Convert => self.convert_file(&params).await,
            FileOperation::Compress => Err(anyhow!("压缩功能尚未实现")),
            FileOperation::Decompress => Err(anyhow!("解压功能尚未实现")),
            FileOperation::Rename => Err(anyhow!("重命名功能尚未实现")),
            FileOperation::Organize => Err(anyhow!("整理功能尚未实现")),
        };

        match result {
            Ok(response) => Ok(ToolResult {
                success: true,
                data: serde_json::to_value(response)?,
                error: None,
            }),
            Err(e) => {
                error!("文件处理失败: {}", e);
                Ok(ToolResult {
                    success: false,
                    data: serde_json::Value::Null,
                    error: Some(e.to_string()),
                })
            }
        }
    }
}
