use anyhow::Result;
use arrow_flight::{
    flight_service_server::FlightService, Action, ActionType, Criteria, Empty, FlightData,
    FlightDescriptor, FlightInfo, HandshakeRequest, HandshakeResponse, PollInfo, PutResult,
    SchemaResult, Ticket,
};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::{Request, Response, Status, Streaming};

use crate::tools::interface::{Tool, ToolParameters};

/// 工具服务实现
pub struct ToolsFlightService {
    tools: Arc<Mutex<Vec<Box<dyn Tool>>>>,
}

impl ToolsFlightService {
    pub fn new() -> Self {
        Self {
            tools: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn register_tool(&self, tool: Box<dyn Tool>) {
        let mut tools = self.tools.lock().await;
        tools.push(tool);
    }
}

type BoxStream<T> = Pin<Box<dyn Stream<Item = Result<T, Status>> + Send + 'static>>;

#[async_trait]
impl FlightService for ToolsFlightService {
    type HandshakeStream = BoxStream<HandshakeResponse>;
    type ListFlightsStream = BoxStream<FlightInfo>;
    type DoGetStream = BoxStream<FlightData>;
    type DoPutStream = BoxStream<PutResult>;
    type DoActionStream = BoxStream<arrow_flight::Result>;
    type ListActionsStream = BoxStream<ActionType>;
    type DoExchangeStream = BoxStream<FlightData>;

    async fn get_schema(
        &self,
        _request: Request<FlightDescriptor>,
    ) -> Result<Response<SchemaResult>, Status> {
        Err(Status::unimplemented("get_schema is not implemented"))
    }

    async fn get_flight_info(
        &self,
        _request: Request<FlightDescriptor>,
    ) -> Result<Response<FlightInfo>, Status> {
        Err(Status::unimplemented("get_flight_info is not implemented"))
    }

    async fn poll_flight_info(
        &self,
        _request: Request<FlightDescriptor>,
    ) -> Result<Response<PollInfo>, Status> {
        Err(Status::unimplemented("poll_flight_info is not implemented"))
    }

    async fn do_exchange(
        &self,
        _request: Request<Streaming<FlightData>>,
    ) -> Result<Response<Self::DoExchangeStream>, Status> {
        Err(Status::unimplemented("do_exchange is not implemented"))
    }

    async fn handshake(
        &self,
        _request: Request<Streaming<HandshakeRequest>>,
    ) -> Result<Response<Self::HandshakeStream>, Status> {
        let output = futures::stream::once(async move {
            Ok(HandshakeResponse {
                protocol_version: 0,
                payload: vec![].into(),
            })
        });
        Ok(Response::new(Box::pin(output)))
    }

    async fn list_flights(
        &self,
        _request: Request<Criteria>,
    ) -> Result<Response<Self::ListFlightsStream>, Status> {
        let tools = self.tools.lock().await;

        let flights: Vec<Result<FlightInfo, Status>> = tools
            .iter()
            .map(|tool| {
                let name = tool.name().to_string();
                Ok(FlightInfo {
                    flight_descriptor: Some(FlightDescriptor {
                        r#type: 0,
                        cmd: name.as_bytes().to_vec().into(),
                        path: vec![],
                    }),
                    schema: vec![].into(),
                    total_records: -1,
                    total_bytes: -1,
                    endpoint: vec![],
                    app_metadata: vec![].into(),
                    ordered: false,
                })
            })
            .collect();

        let output = futures::stream::iter(flights);
        Ok(Response::new(Box::pin(output)))
    }

    async fn do_get(
        &self,
        request: Request<Ticket>,
    ) -> Result<Response<Self::DoGetStream>, Status> {
        let ticket = request.into_inner();
        let tool_name = String::from_utf8(ticket.ticket.to_vec())
            .map_err(|e| Status::invalid_argument(e.to_string()))?;

        let tools = self.tools.lock().await;
        let tool = tools
            .iter()
            .find(|t| t.name() == tool_name)
            .ok_or_else(|| Status::not_found("Tool not found"))?;

        let name = tool.name().to_string();
        let description = tool.description().to_string();

        let info = serde_json::json!({
            "name": name,
            "description": description,
        });

        let data = FlightData {
            flight_descriptor: Some(FlightDescriptor {
                r#type: 0,
                cmd: tool_name.as_bytes().to_vec().into(),
                path: vec![],
            }),
            data_header: vec![].into(),
            data_body: serde_json::to_vec(&info).unwrap().into(),
            app_metadata: vec![].into(),
        };

        let output = futures::stream::once(async move { Ok(data) });
        Ok(Response::new(Box::pin(output)))
    }

    async fn do_put(
        &self,
        _request: Request<Streaming<FlightData>>,
    ) -> Result<Response<Self::DoPutStream>, Status> {
        Err(Status::unimplemented("do_put is not supported"))
    }

    async fn do_action(
        &self,
        request: Request<Action>,
    ) -> Result<Response<Self::DoActionStream>, Status> {
        let action = request.into_inner();

        if action.r#type != "execute" {
            return Err(Status::invalid_argument("Unsupported action type"));
        }

        let params: ToolParameters = serde_json::from_slice(&action.body.to_vec())
            .map_err(|e| Status::invalid_argument(e.to_string()))?;

        // 克隆参数以避免借用问题
        let params_name = params.name.clone();

        let tools = self.tools.lock().await;
        let tool = tools
            .iter()
            .find(|t| t.name() == params_name)
            .ok_or_else(|| Status::not_found("Tool not found"))?;

        // 执行工具并获取结果
        let result = tool
            .execute(params)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let flight_result = arrow_flight::Result {
            body: serde_json::to_vec(&result).unwrap().into(),
        };

        let output = futures::stream::once(async move { Ok(flight_result) });
        Ok(Response::new(Box::pin(output)))
    }

    async fn list_actions(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<Self::ListActionsStream>, Status> {
        let actions = vec![Ok(ActionType {
            r#type: "execute".to_string(),
            description: "Execute a tool".to_string(),
        })];

        let output = futures::stream::iter(actions);
        Ok(Response::new(Box::pin(output)))
    }
}
