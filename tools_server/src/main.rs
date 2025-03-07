use anyhow::Result;
use arrow_flight::flight_service_server::FlightServiceServer;
use async_trait::async_trait;
use rust_agent_core::{
    logging::{init_logger, LoggerConfig},
    tools::interface::{Tool, ToolParameters, ToolResult},
    tools::rpc::server::ToolsFlightService,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tonic::transport::Server;
use tracing::{debug, error, info, Level};

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
        info!("开始分析目录: {}", path.display());
        let mut analysis = FileAnalysis {
            total_size: 0,
            file_count: 0,
            extension_stats: HashMap::new(),
            largest_files: Vec::new(),
        };

        let mut dir_entries = match tokio::fs::read_dir(path).await {
            Ok(entries) => entries,
            Err(e) => {
                error!("无法读取目录 {}: {}", path.display(), e);
                return Err(e.into());
            }
        };

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
                    *analysis.extension_stats.entry(ext.clone()).or_insert(0) += 1;
                    debug!("发现文件类型: {}", ext);
                }

                // 记录大文件
                let path_str = entry.path().to_string_lossy().to_string();
                analysis.largest_files.push((path_str.clone(), file_size));
                analysis
                    .largest_files
                    .sort_by_key(|(_, size)| std::cmp::Reverse(*size));
                analysis.largest_files.truncate(5); // 只保留最大的5个文件
                debug!("处理文件: {} ({} bytes)", path_str, file_size);
            } else if metadata.is_dir() && recursive {
                // 递归分析子目录
                let entry_path = entry.path();
                info!("递归进入子目录: {}", entry_path.display());
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

        info!(
            "目录分析完成: {}, 共 {} 个文件, 总大小 {} bytes",
            path.display(),
            analysis.file_count,
            analysis.total_size
        );
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
        info!("执行文件分析工具，参数: {:?}", params);

        // 解析参数
        let params: FileAnalyzerParams = match serde_json::from_value(params.args.clone()) {
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

        // 分析目录
        let path = Path::new(&params.path);
        info!(
            "开始分析路径: {}, 递归: {}",
            path.display(),
            params.recursive
        );

        match self.analyze_directory(path, params.recursive).await {
            Ok(analysis) => {
                info!("分析成功完成");
                Ok(ToolResult {
                    success: true,
                    data: serde_json::to_value(analysis)?,
                    error: None,
                })
            }
            Err(e) => {
                error!("分析失败: {}", e);
                Ok(ToolResult {
                    success: false,
                    data: serde_json::Value::Null,
                    error: Some(e.to_string()),
                })
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // 创建日志目录
    tokio::fs::create_dir_all("logs").await?;

    // 初始化日志系统
    let log_config =
        LoggerConfig::new("logs", "tools_server", Level::DEBUG).with_console_output(false); // 可以通过设置 false 来禁用终端输出

    // 初始化日志系统
    if let Err(e) = init_logger(log_config) {
        eprintln!("日志系统初始化失败: {}", e);
        return Err(anyhow::anyhow!("日志系统初始化失败: {}", e));
    }

    info!("工具服务器正在启动...");

    // 创建服务实例
    let service = ToolsFlightService::new();

    // 注册文件分析工具
    service
        .register_tool(Box::new(FileAnalyzerTool::new()))
        .await;
    info!("已注册文件分析工具");

    // 启动服务器
    let addr = "[::1]:50051".parse()?;
    info!("工具服务器开始监听地址: {}", addr);

    match Server::builder()
        .add_service(FlightServiceServer::new(service))
        .serve(addr)
        .await
    {
        Ok(_) => info!("服务器正常关闭"),
        Err(e) => error!("服务器运行出错: {}", e),
    }

    Ok(())
}
