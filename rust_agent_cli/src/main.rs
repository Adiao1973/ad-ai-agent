mod chat;
mod config;
mod ui;

use anyhow::Result;
use chat::ChatSession;
use colored::Colorize;
use rust_agent_core::{
    logging::{init_logger, LoggerConfig},
    tools::ToolsClient,
};
use std::io::{self, Write};
use tracing::{error, info, warn, Level};

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志系统
    let log_config =
        LoggerConfig::new("logs", "rust_agent_cli", Level::DEBUG).with_console_output(false); // CLI 程序不需要在控制台显示日志

    if let Err(e) = init_logger(log_config) {
        eprintln!("日志系统初始化失败: {}", e);
        return Err(anyhow::anyhow!("日志系统初始化失败: {}", e));
    }

    info!("Starting Rust Agent CLI...");
    let config = config::get_config();

    let api_key = match config.api_key {
        Some(key) => key,
        None => {
            let key = ui::get_user_input("请输入你的 Deepseek API Key")?;
            info!("API key provided by user");
            key
        }
    };

    let mut session = ChatSession::new(api_key, config.verbose);

    // 尝试连接工具服务
    let tools_addr = config
        .tools_addr
        .unwrap_or_else(|| "http://[::1]:50051".to_string());
    match ToolsClient::connect(&tools_addr).await {
        Ok(client) => {
            session.set_tools_client(client);
            info!("Connected to tools service at {}", tools_addr);
            ui::print_debug("已连接到工具服务");

            // 添加系统提示，告知 AI 可以使用工具
            session.add_system_message(
                "你可以使用以下工具来辅助完成任务：

1. 文件分析工具 (file_analyzer)：
   - 功能：分析指定目录下的文件信息，包括大小、类型统计等
   - 参数：
     - path: 要分析的目录路径（字符串）
     - recursive: 是否递归分析子目录（布尔值）
   - 示例：
   ```tool
   {\"name\": \"file_analyzer\", \"args\": {\"path\": \"/tmp\", \"recursive\": true}}
   ```
   - 返回信息：
     - total_size: 总文件大小
     - file_count: 文件数量
     - extension_stats: 文件扩展名统计
     - largest_files: 最大的5个文件

2. 网络搜索工具 (web_search)：
   - 功能：在互联网上搜索信息，返回相关结果
   - 参数：
     - query: 搜索查询词（字符串）
     - max_results: 最大结果数量（可选，默认5）
   - 示例：
   ```tool
   {\"name\": \"web_search\", \"args\": {\"query\": \"Rust 编程语言\", \"max_results\": 5}}
   ```
   - 返回信息：
     - query: 搜索查询词
     - results: 搜索结果列表，每个结果包含：
       - title: 标题
       - link: 链接
       - snippet: 摘要

注意事项：
1. 工具调用必须使用上述 JSON 格式
2. 参数名称和类型必须严格匹配
3. 每个工具都有特定的用途，请根据实际需求选择合适的工具
4. 如果工具执行失败，会返回错误信息"
                    .to_string(),
            );
        }
        Err(e) => {
            warn!("Failed to connect to tools service: {}", e);
            ui::print_debug(&format!("无法连接到工具服务: {}", e));
            ui::print_debug("将以普通对话模式运行");
        }
    }

    ui::print_welcome();

    loop {
        let user_input = ui::get_user_input("你")?;

        if user_input.to_lowercase() == "quit" || user_input.to_lowercase() == "exit" {
            break;
        }

        info!("User input: {}", user_input);
        session.add_user_message(user_input);

        // 创建加载动画
        let spinner = ui::create_spinner("Deepseek: 思考中...", true);
        let mut is_first_chunk = true;

        match session
            .get_response_stream(|chunk| {
                if is_first_chunk {
                    spinner.finish_and_clear(); // 在第一个响应到达时清除加载动画
                    print!("{}: {}", "Deepseek".blue(), chunk);
                    is_first_chunk = false;
                } else {
                    print!("{}", chunk);
                }
                io::stdout().flush().unwrap();
            })
            .await
        {
            Ok(response) => {
                println!();
                info!("Assistant response received");
                session.add_assistant_message(response);
            }
            Err(e) => {
                spinner.finish_and_clear(); // 确保在出错时也清除加载动画
                println!();
                error!("Failed to get assistant response: {}", e);
                ui::print_error(&e.to_string());
                session.remove_last_message();
            }
        }

        if session.is_verbose() {
            info!("Message count: {}", session.message_count());
            ui::print_debug(&format!("{} 条对话历史", session.message_count()));
        }
    }

    info!("Shutting down Rust Agent CLI...");
    ui::print_goodbye();
    Ok(())
}
