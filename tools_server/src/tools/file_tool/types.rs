use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct FileToolParams {
    pub operation: FileOperation,
    pub input: String,
    #[serde(default)]
    pub output: Option<String>,
    #[serde(default)]
    pub options: Option<ConvertOptions>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileOperation {
    Convert,
    Compress,
    Decompress,
    Rename,
    Organize,
}

#[derive(Debug, Deserialize)]
pub struct ConvertOptions {
    pub format: String,
    #[serde(default)]
    pub quality: Option<String>,
    #[serde(default)]
    pub page_range: Option<String>,
    #[serde(default)]
    pub extra_args: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct FileToolResponse {
    pub success: bool,
    pub message: String,
    pub output_path: Option<String>,
    pub details: Option<FileDetails>,
}

#[derive(Debug, Serialize)]
pub struct FileDetails {
    pub original_size: u64,
    pub processed_size: u64,
    pub processing_time: f64,
}
