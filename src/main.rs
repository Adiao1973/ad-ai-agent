mod api;
mod chat;
mod config;
mod tools;
mod ui;

use anyhow::Result;
use chat::ChatSession;
use tools::ToolsClient;

#[tokio::main]
async fn main() -> Result<()> {
    let config = config::get_config();

    let api_key = match config.api_key {
        Some(key) => key,
        None => ui::get_user_input("请输入你的 Deepseek API Key")?,
    };

    let mut session = ChatSession::new(api_key, config.verbose);

    // 尝试连接工具服务
    let tools_addr = config
        .tools_addr
        .unwrap_or_else(|| "http://[::1]:50051".to_string());
    match ToolsClient::connect(&tools_addr).await {
        Ok(client) => {
            session.set_tools_client(client);
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

        session.add_user_message(user_input);

        let pb = ui::create_progress_bar("等待 Deepseek 响应中...");
        match session.get_response().await {
            Ok(response) => {
                pb.finish_and_clear();
                ui::print_assistant_message(&response);
                session.add_assistant_message(response);
            }
            Err(e) => {
                pb.finish_and_clear();
                ui::print_error(&e.to_string());
                session.remove_last_message();
            }
        }

        if session.is_verbose() {
            ui::print_debug(&format!("{} 条对话历史", session.message_count()));
        }
    }

    ui::print_goodbye();
    Ok(())
}
