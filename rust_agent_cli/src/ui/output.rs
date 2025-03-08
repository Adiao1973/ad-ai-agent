use colored::*;

pub fn print_welcome() {
    println!("欢迎使用 Rust Agent CLI！输入 quit 或 exit 退出程序。");
}

pub fn print_goodbye() {
    println!("感谢使用 Rust Agent CLI，再见！");
}

pub fn print_assistant_message(message: &str) {
    println!("{}: {}", "Deepseek".blue(), message);
}

pub fn print_error(message: &str) {
    eprintln!("{} {}", "错误:".red().bold(), message);
}

pub fn print_debug(message: &str) {
    println!("{} {}", "调试:".yellow().bold(), message);
}
