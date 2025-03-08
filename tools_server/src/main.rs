mod tools;

use anyhow::Result;
use arrow_flight::flight_service_server::FlightServiceServer;
use rust_agent_core::{
    logging::{init_logger, LoggerConfig},
    tools::rpc::server::ToolsFlightService,
};
use tonic::transport::Server;
use tracing::{error, info, Level};

use crate::tools::{FileAnalyzerTool, FileTool, WebSearchTool};

#[tokio::main]
async fn main() -> Result<()> {
    // 创建日志目录
    tokio::fs::create_dir_all("logs").await?;

    // 初始化日志系统
    let log_config =
        LoggerConfig::new("logs", "tools_server", Level::DEBUG).with_console_output(true);

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

    // 注册文件处理工具
    if let Ok(file_tool) = FileTool::new() {
        service.register_tool(Box::new(file_tool)).await;
        info!("已注册文件处理工具");
    } else {
        error!("文件处理工具初始化失败");
    }

    // 注册网络搜索工具
    service.register_tool(Box::new(WebSearchTool::new())).await;
    info!("已注册网络搜索工具");

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
