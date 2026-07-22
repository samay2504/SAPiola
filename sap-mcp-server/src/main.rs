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
use sapiola::v1::storage_service_client::StorageServiceClient;
use sapiola::v1::{
    CypherQueryRequest, GetVertexRequest, GetNeighborsRequest, ListSchemaRequest
};
use tonic::transport::Channel;
use std::collections::{HashMap, HashSet};

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

#[derive(Serialize)]
struct WriteVertexRequest {
    tenant_id: String,
    principal: String,
    node_id: u64,
    properties: HashMap<String, String>,
}

#[derive(Serialize)]
struct WriteEdgeRequest {
    tenant_id: String,
    principal: String,
    source_id: u64,
    target_id: u64,
    properties: HashMap<String, String>,
}

#[derive(Deserialize)]
struct WriteResponse {
    success: bool,
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

    async fn put_vertex(&self, req: WriteVertexRequest) -> anyhow::Result<bool> {
        let res = self.client.post(format!("{}/write/vertex", self.base_url))
            .json(&req)
            .send()
            .await?;
        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            anyhow::bail!("RAG endpoint returned {}: {}", status, text);
        }
        let parsed: WriteResponse = res.json().await?;
        Ok(parsed.success)
    }

    async fn put_edge(&self, req: WriteEdgeRequest) -> anyhow::Result<bool> {
        let res = self.client.post(format!("{}/write/edge", self.base_url))
            .json(&req)
            .send()
            .await?;
        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            anyhow::bail!("RAG endpoint returned {}: {}", status, text);
        }
        let parsed: WriteResponse = res.json().await?;
        Ok(parsed.success)
    }
}

#[derive(Clone)]
struct SapMcpServer {
    graph_client: GraphServiceClient<Channel>,
    storage_client: StorageServiceClient<Channel>,
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

#[derive(Deserialize, schemars::JsonSchema)]
struct GetVertexArgs {
    node_id: u64,
    tenant_id: String,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct GetNeighborsArgs {
    node_id: u64,
    tenant_id: String,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct ExpandNeighborhoodArgs {
    anchor: u64,
    tenant_id: String,
    max_depth: u8,
    max_nodes_visited: usize,
    /// Direction: "Outbound", "Inbound", or "Both". Defaults to "Outbound".
    direction: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct ListSchemaArgs {
    tenant_id: String,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct PutVertexArgs {
    node_id: u64,
    tenant_id: String,
    principal: String,
    properties: HashMap<String, String>,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct PutEdgeArgs {
    source_id: u64,
    target_id: u64,
    tenant_id: String,
    principal: String,
    properties: HashMap<String, String>,
}

#[tool_router]
impl SapMcpServer {
    fn new(graph_client: GraphServiceClient<Channel>, storage_client: StorageServiceClient<Channel>, rag_client: RagHttpClient) -> Self {
        Self {
            graph_client,
            storage_client,
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

    #[tool(description = "Get a single vertex by ID from the graph")]
    async fn get_vertex(&self, Parameters(args): Parameters<GetVertexArgs>) -> Result<CallToolResult, McpError> {
        let mut client = self.storage_client.clone();
        let mut request = tonic::Request::new(GetVertexRequest { node_id: args.node_id });
        request.metadata_mut().insert(
            "tenant-id",
            tonic::metadata::MetadataValue::try_from(&args.tenant_id).unwrap_or_else(|_| tonic::metadata::MetadataValue::from_static("invalid")),
        );

        let response = client.get_vertex(request).await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?
            .into_inner();
            
        let json_result = serde_json::to_string(&response)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            
        Ok(CallToolResult::success(vec![Content::text(json_result)]))
    }

    #[tool(description = "Get inbound and outbound neighbors of a vertex")]
    async fn get_neighbors(&self, Parameters(args): Parameters<GetNeighborsArgs>) -> Result<CallToolResult, McpError> {
        let mut client = self.storage_client.clone();
        let mut request = tonic::Request::new(GetNeighborsRequest { node_id: args.node_id });
        request.metadata_mut().insert(
            "tenant-id",
            tonic::metadata::MetadataValue::try_from(&args.tenant_id).unwrap_or_else(|_| tonic::metadata::MetadataValue::from_static("invalid")),
        );

        let response = client.get_neighbors(request).await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?
            .into_inner();
            
        let json_result = serde_json::to_string(&response)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            
        Ok(CallToolResult::success(vec![Content::text(json_result)]))
    }

    #[tool(description = "List all available node and edge labels")]
    async fn list_schema(&self, Parameters(args): Parameters<ListSchemaArgs>) -> Result<CallToolResult, McpError> {
        let mut client = self.graph_client.clone();
        let mut request = tonic::Request::new(ListSchemaRequest {});
        request.metadata_mut().insert(
            "tenant-id",
            tonic::metadata::MetadataValue::try_from(&args.tenant_id).unwrap_or_else(|_| tonic::metadata::MetadataValue::from_static("invalid")),
        );

        let response = client.list_schema(request).await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?
            .into_inner();
            
        let json_result = serde_json::to_string(&response)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            
        Ok(CallToolResult::success(vec![Content::text(json_result)]))
    }

    #[tool(description = "Put a vertex into the graph (Requires write authorization)")]
    async fn put_vertex(&self, Parameters(args): Parameters<PutVertexArgs>) -> Result<CallToolResult, McpError> {
        let req = WriteVertexRequest {
            tenant_id: args.tenant_id,
            principal: args.principal,
            node_id: args.node_id,
            properties: args.properties,
        };
        let success = self.rag_client.put_vertex(req).await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            
        Ok(CallToolResult::success(vec![Content::text(format!("Success: {}", success))]))
    }

    #[tool(description = "Put an edge into the graph (Requires write authorization)")]
    async fn put_edge(&self, Parameters(args): Parameters<PutEdgeArgs>) -> Result<CallToolResult, McpError> {
        let req = WriteEdgeRequest {
            tenant_id: args.tenant_id,
            principal: args.principal,
            source_id: args.source_id,
            target_id: args.target_id,
            properties: args.properties,
        };
        let success = self.rag_client.put_edge(req).await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            
        Ok(CallToolResult::success(vec![Content::text(format!("Success: {}", success))]))
    }

    #[tool(description = "Perform a bounded BFS traversal")]
    async fn expand_neighborhood(&self, Parameters(args): Parameters<ExpandNeighborhoodArgs>) -> Result<CallToolResult, McpError> {
        let mut visited = HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        let mut result_graph = HashMap::new();
        
        let direction = args.direction.unwrap_or_else(|| "Outbound".to_string()).to_lowercase();
        
        queue.push_back((args.anchor, 0));
        
        while let Some((curr, depth)) = queue.pop_front() {
            if visited.contains(&curr) || visited.len() >= args.max_nodes_visited {
                continue;
            }
            
            visited.insert(curr);
            
            let mut request = tonic::Request::new(GetNeighborsRequest { node_id: curr });
            request.metadata_mut().insert(
                "tenant-id",
                tonic::metadata::MetadataValue::try_from(&args.tenant_id).unwrap_or_else(|_| tonic::metadata::MetadataValue::from_static("invalid")),
            );
            
            let mut client = self.storage_client.clone();
            if let Ok(resp) = client.get_neighbors(request).await {
                let edges = resp.into_inner();
                
                let mut out_edges = edges.out_edges;
                let mut in_edges = edges.in_edges;
                
                if direction == "inbound" {
                    out_edges.clear();
                } else if direction == "outbound" {
                    in_edges.clear();
                }
                
                let mut neighbors = Vec::new();
                neighbors.extend(out_edges.iter().cloned());
                neighbors.extend(in_edges.iter().cloned());
                
                result_graph.insert(curr, (out_edges, in_edges));
                
                if depth < args.max_depth {
                    for n in neighbors {
                        if !visited.contains(&n) {
                            queue.push_back((n, depth + 1));
                        }
                    }
                }
            }
        }
        
        let json_result = serde_json::to_string(&result_graph)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            
        Ok(CallToolResult::success(vec![Content::text(json_result)]))
    }
}

#[tool_handler]
impl ServerHandler for SapMcpServer {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let graph_url = std::env::var("SAPIOLA_GRAPH_URL").unwrap_or_else(|_| "http://127.0.0.1:50053".to_string());
    let rag_url = std::env::var("SAPIOLA_RAG_URL").unwrap_or_else(|_| "http://127.0.0.1:8000".to_string());

    // Connect gRPC
    let graph_client = GraphServiceClient::connect(graph_url.clone()).await?;
    let storage_client = StorageServiceClient::connect(graph_url).await?;

    let app = SapMcpServer::new(graph_client, storage_client, RagHttpClient::new(rag_url));

    app.serve(stdio()).await?.waiting().await?;
    Ok(())
}
