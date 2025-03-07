use anyhow::Result;
use arrow_flight::{
    flight_service_server::{FlightService, FlightServiceServer},
    Action, ActionType, Criteria, Empty, FlightData, FlightDescriptor, FlightInfo,
    HandshakeRequest, HandshakeResponse, PollInfo, PutResult, SchemaResult, Ticket,
};
use async_trait::async_trait;
use futures::Stream;
use rust_agent_core::tools::interface::{Tool, ToolParameters, ToolResult};
use rust_agent_core::tools::rpc::server::ToolsFlightService;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::{transport::Server, Request, Response, Status, Streaming};

// 文件分析工具实现
#[derive(Debug, Serialize, Deserialize)]
struct FileAnalyzerParams {
    path: String,
    recursive: bool,
}

#[derive(Debug, Serialize)]
struct FileAnalysis {
    total_size: u64,
    file_count: usize,
    extension_stats: HashMap<String, usize>,
    largest_files: Vec<(String, u64)>,
}

struct FileAnalyzerTool;

impl FileAnalyzerTool {
    fn new() -> Self {
        Self
    }

    async fn analyze_directory(&self, path: &Path, recursive: bool) -> Result<FileAnalysis> {
        let mut analysis = FileAnalysis {
            total_size: 0,
            file_count: 0,
            extension_stats: HashMap::new(),
            largest_files: Vec::new(),
        };

        let mut dir_entries = tokio::fs::read_dir(path).await?;
        while let Some(entry) = dir_entries.next_entry().await? {
            let metadata = entry.metadata().await?;

            if metadata.is_file() {
                // 更新文件计数
                analysis.file_count += 1;

                // 更新总大小
                let file_size = metadata.len();
                analysis.total_size += file_size;

                // 统计文件扩展名
                if let Some(ext) = entry.path().extension() {
                    let ext = ext.to_string_lossy().to_string();
                    *analysis.extension_stats.entry(ext).or_insert(0) += 1;
                }

                // 记录大文件
                let path_str = entry.path().to_string_lossy().to_string();
                analysis.largest_files.push((path_str, file_size));
                analysis
                    .largest_files
                    .sort_by_key(|(_, size)| std::cmp::Reverse(*size));
                analysis.largest_files.truncate(5); // 只保留最大的5个文件
            } else if metadata.is_dir() && recursive {
                // 递归分析子目录 - 使用 Box::pin 避免无限大小的 Future
                let entry_path = entry.path();
                let sub_analysis_future = self.analyze_directory(&entry_path, recursive);
                let sub_analysis = Box::pin(sub_analysis_future).await?;

                analysis.total_size += sub_analysis.total_size;
                analysis.file_count += sub_analysis.file_count;

                // 合并扩展名统计
                for (ext, count) in sub_analysis.extension_stats {
                    *analysis.extension_stats.entry(ext).or_insert(0) += count;
                }

                // 更新最大文件列表
                analysis.largest_files.extend(sub_analysis.largest_files);
                analysis
                    .largest_files
                    .sort_by_key(|(_, size)| std::cmp::Reverse(*size));
                analysis.largest_files.truncate(5);
            }
        }

        Ok(analysis)
    }
}

#[async_trait]
impl Tool for FileAnalyzerTool {
    fn name(&self) -> &str {
        "file_analyzer"
    }

    fn description(&self) -> &str {
        "分析指定目录下的文件信息，包括大小、类型统计等"
    }

    async fn execute(&self, params: ToolParameters) -> Result<ToolResult> {
        // 解析参数
        let params: FileAnalyzerParams = serde_json::from_value(params.args)?;

        // 分析目录
        let path = Path::new(&params.path);
        match self.analyze_directory(path, params.recursive).await {
            Ok(analysis) => Ok(ToolResult {
                success: true,
                data: serde_json::to_value(analysis)?,
                error: None,
            }),
            Err(e) => Ok(ToolResult {
                success: false,
                data: serde_json::Value::Null,
                error: Some(e.to_string()),
            }),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // 创建服务实例
    let service = ToolsFlightService::new();

    // 注册文件分析工具
    service
        .register_tool(Box::new(FileAnalyzerTool::new()))
        .await;

    // 启动服务器
    let addr = "[::1]:50051".parse()?;
    println!("工具服务器正在启动，监听地址: {}", addr);

    Server::builder()
        .add_service(FlightServiceServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}
