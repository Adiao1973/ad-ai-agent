use anyhow::Result;
use async_trait::async_trait;
use rust_agent_core::tools::interface::{Tool, ToolParameters, ToolResult};
use serde::{Deserialize, Serialize};
use tracing::{error, info};

#[derive(Debug, Serialize, Deserialize)]
pub struct WebSearchParams {
    query: String,
    max_results: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct SearchResult {
    title: String,
    link: String,
    snippet: String,
}

#[derive(Debug, Serialize)]
pub struct WebSearchResult {
    query: String,
    results: Vec<SearchResult>,
}

pub struct WebSearchTool;

impl WebSearchTool {
    pub fn new() -> Self {
        Self
    }

    async fn perform_search(&self, query: &str, max_results: usize) -> Result<Vec<SearchResult>> {
        // 构建 DuckDuckGo API URL
        let encoded_query = urlencoding::encode(query);
        let url = format!(
            "https://api.duckduckgo.com/?q={}&format=json&no_html=1&skip_disambig=1",
            encoded_query
        );

        // 创建支持代理的客户端
        let client = reqwest::Client::builder()
            .proxy(reqwest::Proxy::all("http://127.0.0.1:7890")?) // 设置代理
            .build()?;

        // 发送请求
        let response = client
            .get(&url)
            .header("User-Agent", "RustAgent/1.0")
            .send()
            .await?;

        // 解析响应
        #[derive(Deserialize)]
        struct DuckDuckGoResult {
            #[serde(rename = "AbstractText")]
            abstract_text: String,
            #[serde(rename = "AbstractURL")]
            abstract_url: String,
            #[serde(rename = "RelatedTopics")]
            related_topics: Vec<Topic>,
        }

        #[derive(Deserialize)]
        struct Topic {
            #[serde(rename = "Text")]
            text: Option<String>,
            #[serde(rename = "FirstURL")]
            url: Option<String>,
        }

        // 打印响应状态和内容以便调试
        info!("搜索响应状态: {}", response.status());
        let text = response.text().await?;
        info!("搜索响应内容: {}", text);

        // 解析响应
        let ddg_result: DuckDuckGoResult = serde_json::from_str(&text)?;
        let mut results = Vec::new();

        // 添加主要结果
        if !ddg_result.abstract_text.is_empty() {
            results.push(SearchResult {
                title: "主要结果".to_string(),
                link: ddg_result.abstract_url,
                snippet: ddg_result.abstract_text,
            });
        }

        // 添加相关主题
        for topic in ddg_result
            .related_topics
            .iter()
            .take(max_results.saturating_sub(1))
        {
            if let (Some(text), Some(url)) = (&topic.text, &topic.url) {
                results.push(SearchResult {
                    title: text.clone(),
                    link: url.clone(),
                    snippet: text.clone(),
                });
            }
        }

        Ok(results)
    }
}

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        "在互联网上搜索信息，返回相关结果"
    }

    async fn execute(&self, params: ToolParameters) -> Result<ToolResult> {
        info!("执行网络搜索工具，参数: {:?}", params);

        // 解析参数
        let params: WebSearchParams = match serde_json::from_value(params.args.clone()) {
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

        // 设置默认最大结果数
        let max_results = params.max_results.unwrap_or(5);

        match self.perform_search(&params.query, max_results).await {
            Ok(results) => {
                let search_result = WebSearchResult {
                    query: params.query,
                    results,
                };

                info!("搜索成功完成，找到 {} 个结果", search_result.results.len());
                Ok(ToolResult {
                    success: true,
                    data: serde_json::to_value(search_result)?,
                    error: None,
                })
            }
            Err(e) => {
                error!("搜索失败: {}", e);
                Ok(ToolResult {
                    success: false,
                    data: serde_json::Value::Null,
                    error: Some(e.to_string()),
                })
            }
        }
    }
}
