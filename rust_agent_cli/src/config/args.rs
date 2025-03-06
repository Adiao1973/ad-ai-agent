use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Deepseek API Key
    #[arg(short, long)]
    pub api_key: Option<String>,

    /// 是否显示详细信息
    #[arg(short, long, default_value_t = false)]
    pub verbose: bool,

    /// 工具服务地址
    #[arg(short, long)]
    pub tools_addr: Option<String>,
}

impl Args {
    pub fn new() -> Self {
        Self::parse()
    }
}
