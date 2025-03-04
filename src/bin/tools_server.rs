use anyhow::Result;
use arrow_flight::{
    flight_service_server::{FlightService, FlightServiceServer},
    Action, ActionType, Criteria, Empty, FlightData, FlightDescriptor, FlightInfo,
    HandshakeRequest, HandshakeResponse, PollInfo, PutResult, SchemaResult, Ticket,
};
use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::{transport::Server, Request, Response, Status, Streaming};

// 直接在二进制文件中定义所需的工具接口和实现
// 工具接口
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolParameters {
    pub name: String,
    pub args: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub data: serde_json::Value,
    pub error: Option<String>,
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    async fn execute(&self, params: ToolParameters) -> Result<ToolResult>;
}

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

// 工具服务实现
pub struct ToolsFlightService {
    tools: Arc<Mutex<Vec<Box<dyn Tool>>>>,
}

impl ToolsFlightService {
    pub fn new() -> Self {
        Self {
            tools: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn register_tool(&self, tool: Box<dyn Tool>) {
        let mut tools = self.tools.lock().await;
        tools.push(tool);
    }
}

type BoxStream<T> = Pin<Box<dyn Stream<Item = Result<T, Status>> + Send + 'static>>;

#[async_trait]
impl FlightService for ToolsFlightService {
    type HandshakeStream = BoxStream<HandshakeResponse>;
    type ListFlightsStream = BoxStream<FlightInfo>;
    type DoGetStream = BoxStream<FlightData>;
    type DoPutStream = BoxStream<PutResult>;
    type DoActionStream = BoxStream<arrow_flight::Result>;
    type ListActionsStream = BoxStream<ActionType>;
    type DoExchangeStream = BoxStream<FlightData>;

    async fn get_schema(
        &self,
        _request: Request<FlightDescriptor>,
    ) -> Result<Response<SchemaResult>, Status> {
        Err(Status::unimplemented("get_schema is not implemented"))
    }

    async fn get_flight_info(
        &self,
        _request: Request<FlightDescriptor>,
    ) -> Result<Response<FlightInfo>, Status> {
        Err(Status::unimplemented("get_flight_info is not implemented"))
    }

    async fn poll_flight_info(
        &self,
        _request: Request<FlightDescriptor>,
    ) -> Result<Response<PollInfo>, Status> {
        Err(Status::unimplemented("poll_flight_info is not implemented"))
    }

    async fn do_exchange(
        &self,
        _request: Request<Streaming<FlightData>>,
    ) -> Result<Response<Self::DoExchangeStream>, Status> {
        Err(Status::unimplemented("do_exchange is not implemented"))
    }

    async fn handshake(
        &self,
        _request: Request<Streaming<HandshakeRequest>>,
    ) -> Result<Response<Self::HandshakeStream>, Status> {
        let output = futures::stream::once(async move {
            Ok(HandshakeResponse {
                protocol_version: 0,
                payload: vec![].into(),
            })
        });
        Ok(Response::new(Box::pin(output)))
    }

    async fn list_flights(
        &self,
        _request: Request<Criteria>,
    ) -> Result<Response<Self::ListFlightsStream>, Status> {
        let tools = self.tools.lock().await;

        let flights: Vec<Result<FlightInfo, Status>> = tools
            .iter()
            .map(|tool| {
                let name = tool.name().to_string();
                Ok(FlightInfo {
                    flight_descriptor: Some(FlightDescriptor {
                        r#type: 0,
                        cmd: name.as_bytes().to_vec().into(),
                        path: vec![],
                    }),
                    schema: vec![].into(),
                    total_records: -1,
                    total_bytes: -1,
                    endpoint: vec![],
                    app_metadata: vec![].into(),
                    ordered: false,
                })
            })
            .collect();

        let output = futures::stream::iter(flights);
        Ok(Response::new(Box::pin(output)))
    }

    async fn do_get(
        &self,
        request: Request<Ticket>,
    ) -> Result<Response<Self::DoGetStream>, Status> {
        let ticket = request.into_inner();
        let tool_name = String::from_utf8(ticket.ticket.to_vec())
            .map_err(|e| Status::invalid_argument(e.to_string()))?;

        let tools = self.tools.lock().await;
        let tool = tools
            .iter()
            .find(|t| t.name() == tool_name)
            .ok_or_else(|| Status::not_found("Tool not found"))?;

        let name = tool.name().to_string();
        let description = tool.description().to_string();

        let info = serde_json::json!({
            "name": name,
            "description": description,
        });

        let data = FlightData {
            flight_descriptor: Some(FlightDescriptor {
                r#type: 0,
                cmd: tool_name.as_bytes().to_vec().into(),
                path: vec![],
            }),
            data_header: vec![].into(),
            data_body: serde_json::to_vec(&info).unwrap().into(),
            app_metadata: vec![].into(),
        };

        let output = futures::stream::once(async move { Ok(data) });
        Ok(Response::new(Box::pin(output)))
    }

    async fn do_put(
        &self,
        _request: Request<Streaming<FlightData>>,
    ) -> Result<Response<Self::DoPutStream>, Status> {
        Err(Status::unimplemented("do_put is not supported"))
    }

    async fn do_action(
        &self,
        request: Request<Action>,
    ) -> Result<Response<Self::DoActionStream>, Status> {
        let action = request.into_inner();

        if action.r#type != "execute" {
            return Err(Status::invalid_argument("Unsupported action type"));
        }

        let params: ToolParameters = serde_json::from_slice(&action.body.to_vec())
            .map_err(|e| Status::invalid_argument(e.to_string()))?;

        // 克隆参数以避免借用问题
        let params_name = params.name.clone();

        let tools = self.tools.lock().await;
        let tool = tools
            .iter()
            .find(|t| t.name() == params_name)
            .ok_or_else(|| Status::not_found("Tool not found"))?;

        // 执行工具并获取结果
        let result = tool
            .execute(params)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let flight_result = arrow_flight::Result {
            body: serde_json::to_vec(&result).unwrap().into(),
        };

        let output = futures::stream::once(async move { Ok(flight_result) });
        Ok(Response::new(Box::pin(output)))
    }

    async fn list_actions(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<Self::ListActionsStream>, Status> {
        let actions = vec![Ok(ActionType {
            r#type: "execute".to_string(),
            description: "Execute a tool".to_string(),
        })];

        let output = futures::stream::iter(actions);
        Ok(Response::new(Box::pin(output)))
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
