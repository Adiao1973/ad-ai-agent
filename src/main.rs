mod api;
mod chat;
mod config;
mod ui;

use anyhow::Result;
use chat::ChatSession;

#[tokio::main]
async fn main() -> Result<()> {
    let config = config::get_config();

    let api_key = match config.api_key {
        Some(key) => key,
        None => ui::get_user_input("请输入你的 Deepseek API Key")?,
    };

    let mut session = ChatSession::new(api_key, config.verbose);

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
