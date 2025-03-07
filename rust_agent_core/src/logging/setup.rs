//! 日志系统模块
//!
//! 这个模块提供了文档搜索引擎的日志功能，包括：
//! - 日志配置
//! - 日志初始化
//! - 日志文件滚动
//! - 本地时间支持

use std::path::Path;
use tracing::{error, info, warn, Level};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{
    fmt, fmt::time::LocalTime, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter,
};

/// 日志配置结构体
#[derive(Debug)]
pub struct LoggerConfig {
    /// 日志文件目录
    log_dir: String,
    /// 日志文件名前缀
    file_prefix: String,
    /// 默认日志级别
    level: Level,
    /// 是否输出到终端
    console_output: bool,
}

impl Default for LoggerConfig {
    fn default() -> Self {
        Self {
            log_dir: "logs".to_string(),
            file_prefix: "agent".to_string(),
            level: Level::INFO,
            console_output: true,
        }
    }
}

impl LoggerConfig {
    /// 创建新的日志配置
    pub fn new(log_dir: impl Into<String>, file_prefix: impl Into<String>, level: Level) -> Self {
        Self {
            log_dir: log_dir.into(),
            file_prefix: file_prefix.into(),
            level,
            console_output: true,
        }
    }

    /// 设置日志目录
    pub fn with_log_dir(mut self, log_dir: impl Into<String>) -> Self {
        self.log_dir = log_dir.into();
        self
    }

    /// 设置文件名前缀
    pub fn with_file_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.file_prefix = prefix.into();
        self
    }

    /// 设置日志级别
    pub fn with_level(mut self, level: Level) -> Self {
        self.level = level;
        self
    }

    /// 设置是否输出到终端
    pub fn with_console_output(mut self, enable: bool) -> Self {
        self.console_output = enable;
        self
    }
}

/// 初始化日志系统
///
/// # 参数
///
/// * `config` - 日志配置
///
/// # 返回值
///
/// 返回 Result，成功时返回 ()，失败时返回错误
///
/// # 示例
///
/// ```rust
/// use document_search_engine::utils::logger::{init_logger, LoggerConfig};
/// use tracing::Level;
///
/// let config = LoggerConfig::default()
///     .with_level(Level::DEBUG);
/// init_logger(config).expect("初始化日志失败");
/// ```
pub fn init_logger(config: LoggerConfig) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 确保日志目录存在
    if !Path::new(&config.log_dir).exists() {
        std::fs::create_dir_all(&config.log_dir)?;
    }

    // 创建文件输出
    let file_appender = RollingFileAppender::new(
        Rotation::DAILY,
        &config.log_dir,
        format!("{}.log", config.file_prefix),
    );

    // 创建环境过滤器
    let env_filter = EnvFilter::from_default_env()
        .add_directive(config.level.into())
        .add_directive("agent=debug".parse()?);

    // 创建文件输出层
    let file_layer = fmt::layer()
        .with_thread_ids(true)
        .with_target(false)
        .with_file(true)
        .with_line_number(true)
        .with_thread_names(true)
        .with_ansi(false)
        .with_timer(LocalTime::rfc_3339())
        .with_level(true)
        .with_writer(file_appender);

    // 创建基础订阅者
    let subscriber = tracing_subscriber::registry()
        .with(env_filter)
        .with(file_layer);

    // 如果启用终端输出，添加终端输出层
    if config.console_output {
        let console_layer = fmt::layer()
            .with_thread_ids(true)
            .with_target(false)
            .with_file(true)
            .with_line_number(true)
            .with_thread_names(true)
            .with_ansi(true) // 终端支持 ANSI 颜色
            .with_timer(LocalTime::rfc_3339())
            .with_level(true)
            .with_writer(std::io::stdout);

        subscriber.with(console_layer).try_init()?;
    } else {
        subscriber.try_init()?;
    }

    info!("日志系统初始化完成");
    Ok(())
}
