use colored::*;

pub fn print_welcome() {
    println!("{}", "欢迎使用 Deepseek CLI 对话程序！".green());
    println!("{}", "输入 'quit' 或 'exit' 退出程序".yellow());
}

pub fn print_goodbye() {
    println!("{}", "感谢使用，再见！".green());
}

pub fn print_assistant_message(message: &str) {
    println!("{}: {}", "Deepseek".blue(), message);
}

pub fn print_error(error: &str) {
    eprintln!("{}: {}", "错误".red(), error);
}

pub fn print_debug(message: &str) {
    println!("{}: {}", "调试".yellow(), message);
}
