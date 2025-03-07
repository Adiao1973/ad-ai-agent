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
                "你可以使用以下格式调用工具：\n\
                ```tool\n\
                {\"name\": \"工具名称\", \"args\": {\"参数名\": \"参数值\"}}\n\
                ```\n\
                例如：\n\
                ```tool\n\
                {\"name\": \"file_analyzer\", \"args\": {\"path\": \"/tmp\", \"recursive\": true}}\n\
                ```".to_string()
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

        print!("{}: ", "Deepseek".blue());
        io::stdout().flush()?;

        match session
            .get_response_stream(|chunk| {
                print!("{}", chunk);
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
