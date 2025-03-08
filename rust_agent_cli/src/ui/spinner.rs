use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

/// 创建一个加载动画
///
/// # Arguments
/// * `message` - 显示的消息
/// * `auto_tick` - 是否自动更新动画（默认为 true）
pub fn create_spinner(message: &str, auto_tick: bool) -> ProgressBar {
    let pb = ProgressBar::new_spinner();

    // 设置样式
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template("{spinner:.blue} {msg}")
            .unwrap(),
    );

    // 设置消息
    pb.set_message(message.to_string());

    // 如果启用自动更新，设置更新间隔
    if auto_tick {
        pb.enable_steady_tick(Duration::from_millis(100));
    }

    pb
}

/// 创建一个简单的进度条（为了向后兼容，实际上也是加载动画）
#[deprecated(since = "0.1.0", note = "请使用 create_spinner 替代")]
pub fn create_progress_bar(message: &str) -> ProgressBar {
    create_spinner(message, true)
}
