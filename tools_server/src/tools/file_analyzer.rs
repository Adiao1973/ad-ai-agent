use anyhow::Result;
use async_trait::async_trait;
use rust_agent_core::tools::interface::{Tool, ToolParameters, ToolResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tracing::{error, info};

#[derive(Debug, Serialize, Deserialize)]
pub struct FileAnalyzerParams {
    path: String,
    recursive: bool,
}

#[derive(Debug, Serialize)]
pub struct FileAnalysis {
    total_size: u64,
    file_count: usize,
    extension_stats: HashMap<String, usize>,
    largest_files: Vec<(String, u64)>,
}

pub struct FileAnalyzerTool;

impl FileAnalyzerTool {
    pub fn new() -> Self {
        Self
    }

    async fn analyze_directory(&self, path: &Path, recursive: bool) -> Result<FileAnalysis> {
        let mut analysis = FileAnalysis {
            total_size: 0,
            file_count: 0,
            extension_stats: HashMap::new(),
            largest_files: Vec::new(),
        };

        if !path.exists() {
            return Err(anyhow::anyhow!("路径不存在"));
        }

        let mut files_to_process = vec![path.to_path_buf()];

        while let Some(current_path) = files_to_process.pop() {
            if current_path.is_file() {
                if let Ok(metadata) = fs::metadata(&current_path) {
                    let size = metadata.len();
                    analysis.total_size += size;
                    analysis.file_count += 1;

                    // 统计文件扩展名
                    if let Some(ext) = current_path.extension() {
                        let ext_str = ext.to_string_lossy().to_string();
                        *analysis.extension_stats.entry(ext_str).or_insert(0) += 1;
                    }

                    // 记录大文件
                    analysis
                        .largest_files
                        .push((current_path.to_string_lossy().to_string(), size));
                    analysis.largest_files.sort_by(|a, b| b.1.cmp(&a.1));
                    analysis.largest_files.truncate(5);
                }
            } else if current_path.is_dir() && (recursive || current_path == path) {
                if let Ok(entries) = fs::read_dir(&current_path) {
                    for entry in entries.flatten() {
                        files_to_process.push(entry.path());
                    }
                }
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
