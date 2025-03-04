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
}

impl Args {
    pub fn new() -> Self {
        Self::parse()
    }
}
