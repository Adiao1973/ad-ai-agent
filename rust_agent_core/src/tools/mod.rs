pub mod interface;
pub mod parser;
pub mod rpc;

pub use interface::{Tool, ToolParameters, ToolResult};
pub use parser::{format_tool_result, parse_tool_calls};
pub use rpc::client::ToolsClient;
