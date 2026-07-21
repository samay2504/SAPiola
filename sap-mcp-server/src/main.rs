use rmcp::{tool, tool_router, tool_handler, ServerHandler, ServiceExt, model::{CallToolResult, Content}, ErrorData as McpError};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::transport::io::stdio;
use reqwest::Client;
use serde::{Deserialize, Serialize};

pub mod sapiola {
    pub mod v1 {
        tonic::include_proto!("sapiola.v1");
    }
}

use sapiola::v1::graph_service_client::GraphServiceClient;
use sapiola::v1::CypherQueryRequest;
use tonic::transport::Channel;

#[derive(Serialize)]
struct RagQueryRequest {
    query: String,
    principal: String,
}

#[derive(Deserialize)]
struct AnswerResponse {
    answer: String,
    degraded: bool,
}

#[derive(Clone)]
struct RagHttpClient {
    base_url: String,
    client: Client,
}

impl RagHttpClient {
    fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: Client::new(),
        }
    }

    async fn answer(&self, question: String, tenant_id: String) -> anyhow::Result<AnswerResponse> {
        let res = self.client.post(format!("{}/answer", self.base_url))
            .json(&RagQueryRequest {
                query: question,
                principal: tenant_id,
            })
            .send()
            .await?;
            
        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            anyhow::bail!("RAG endpoint returned {}: {}", status, text);
        }
        
        let parsed: AnswerResponse = res.json().await?;
        Ok(parsed)
    }
}

#[derive(Clone)]
struct SapMcpServer {
    graph_client: GraphServiceClient<Channel>,
    rag_client: RagHttpClient,
    tool_router: rmcp::handler::server::tool::ToolRouter<Self>,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct CypherArgs {
    cypher: String,
    tenant_id: String,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct AskSapArgs {
    question: String,
    tenant_id: String,
}

#[tool_router]
impl SapMcpServer {
    fn new(graph_client: GraphServiceClient<Channel>, rag_client: RagHttpClient) -> Self {
        Self {
            graph_client,
            rag_client,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "A Cypher-subset query over the SAP graph")]
    async fn run_graph_query(&self, Parameters(args): Parameters<CypherArgs>) -> Result<CallToolResult, McpError> {
        let mut client = self.graph_client.clone();
        
        let mut request = tonic::Request::new(CypherQueryRequest {
            cypher: args.cypher,
            limit: 0,
        });
        request.metadata_mut().insert(
            "x-tenant-id",
            tonic::metadata::MetadataValue::try_from(&args.tenant_id).unwrap_or_else(|_| tonic::metadata::MetadataValue::from_static("invalid")),
        );

        let response = client.query(request).await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?
            .into_inner();
            
        let json_result = serde_json::to_string(&response)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            
        Ok(CallToolResult::success(vec![Content::text(json_result)]))
    }

    #[tool(description = "Ask SAP questions via the RAG Orchestrator")]
    async fn ask_sap(&self, Parameters(args): Parameters<AskSapArgs>) -> Result<CallToolResult, McpError> {
        let result = self.rag_client.answer(args.question, args.tenant_id).await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            
        let prefix = if result.degraded { "[DEGRADED MODE] " } else { "" };
        let formatted = format!("{}{}", prefix, result.answer);
        
        Ok(CallToolResult::success(vec![Content::text(formatted)]))
    }
}

#[tool_handler]
impl ServerHandler for SapMcpServer {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let graph_url = std::env::var("SAPIOLA_GRAPH_URL").unwrap_or_else(|_| "http://[::1]:50051".to_string());
    let rag_url = std::env::var("SAPIOLA_RAG_URL").unwrap_or_else(|_| "http://127.0.0.1:8000".to_string());

    // Connect gRPC
    let graph_client = GraphServiceClient::connect(graph_url).await?;

    let app = SapMcpServer::new(graph_client, RagHttpClient::new(rag_url));

    app.serve(stdio()).await?.waiting().await?;
    Ok(())
}
