use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;

use crate::tools::interface::{Tool, ToolParameters, ToolResult};

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

        let mut dir_entries = fs::read_dir(path).await?;
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
                analysis
                    .largest_files
                    .push((entry.path().to_string_lossy().to_string(), file_size));
                analysis
                    .largest_files
                    .sort_by_key(|(_, size)| std::cmp::Reverse(*size));
                analysis.largest_files.truncate(5); // 只保留最大的5个文件
            } else if metadata.is_dir() && recursive {
                // 递归分析子目录
                if let Ok(sub_analysis) = self.analyze_directory(&entry.path(), recursive).await {
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
