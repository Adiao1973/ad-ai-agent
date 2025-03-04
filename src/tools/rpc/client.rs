use anyhow::Result;
use arrow_flight::{flight_service_client::FlightServiceClient, Action, Criteria};
use tonic::transport::Channel;

use crate::tools::interface::{ToolParameters, ToolResult};

pub struct ToolsClient {
    client: FlightServiceClient<Channel>,
}

impl ToolsClient {
    pub async fn connect(addr: &str) -> Result<Self> {
        let client = FlightServiceClient::connect(addr.to_string()).await?;
        Ok(Self { client })
    }

    pub async fn list_tools(&mut self) -> Result<Vec<String>> {
        let request = tonic::Request::new(Criteria::default());
        let response = self.client.list_flights(request).await?;
        let mut stream = response.into_inner();

        let mut tools = Vec::new();
        while let Some(flight_info) = stream.message().await? {
            if let Some(descriptor) = flight_info.flight_descriptor {
                let cmd = descriptor.cmd;
                tools.push(String::from_utf8(cmd.to_vec())?);
            }
        }

        Ok(tools)
    }

    pub async fn execute_tool(&mut self, params: ToolParameters) -> Result<ToolResult> {
        let action = Action {
            r#type: "execute".into(),
            body: serde_json::to_vec(&params)?.into(),
        };

        let request = tonic::Request::new(action);
        let response = self.client.do_action(request).await?;
        let mut stream = response.into_inner();

        if let Some(result) = stream.message().await? {
            let tool_result: ToolResult = serde_json::from_slice(&result.body.to_vec())?;
            Ok(tool_result)
        } else {
            anyhow::bail!("No result received from tool execution")
        }
    }
}
