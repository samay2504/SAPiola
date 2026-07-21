use std::sync::Arc;
use tonic::{transport::Server, Request, Response, Status};
use tracing::{info, warn};
use std::env;

use poly_lsm_core::{PolyLsmEngine, GraphStore, pointer_index::PointerIndexStore};

pub mod pb {
    tonic::include_proto!("sapiola.v1");
}
use pb::graph_service_server::{GraphService, GraphServiceServer};
use pb::storage_service_server::{StorageService, StorageServiceServer};
use pb::{
    QueryCandidatesRequest, QueryCandidatesResponse, CandidateInfo, CypherQueryRequest,
    CypherQueryResponse, ListLabelsRequest, ListLabelsResponse, PutEdgeRequest, PutEdgeResponse,
    GetNeighborsRequest, GetNeighborsResponse,
};

pub struct GraphServiceImpl {
    engine: Arc<PolyLsmEngine>,
}

impl GraphServiceImpl {
    pub fn new(engine: Arc<PolyLsmEngine>) -> Self {
        Self { engine }
    }
}

#[tonic::async_trait]
impl GraphService for GraphServiceImpl {
    async fn query_candidates(
        &self,
        request: Request<QueryCandidatesRequest>,
    ) -> Result<Response<QueryCandidatesResponse>, Status> {
        let tenant_id = request
            .metadata()
            .get("tenant-id")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("default")
            .to_string();

        let req = request.into_inner();
        let query = req.query.to_lowercase();
        
        let mut candidates = Vec::new();
        
        if let Ok(vertices) = self.engine.scan_vertices(&tenant_id) {
            for (node_id, props) in vertices {
                let label = props.get("label").cloned().unwrap_or_else(|| "Unknown".to_string());
                
                let mut sap_key = format!("{}-{}", label, node_id);
                let mut matched = false;
                
                for (k, v) in &props {
                    if k.to_lowercase().contains("id") || k.to_lowercase() == "pk" {
                        sap_key = v.clone();
                    }
                    if v.to_lowercase().contains(&query) {
                        matched = true;
                    }
                }
                
                if matched {
                    candidates.push(CandidateInfo {
                        node_id: node_id.try_into().unwrap_or(0),
                        label,
                        sap_key,
                        pointer_path: vec![],
                    });
                    
                    if candidates.len() > 50 {
                        break;
                    }
                }
            }
        }
        
        Ok(Response::new(QueryCandidatesResponse {
            candidates,
            error: None,
        }))
    }

    async fn query(&self, _req: Request<CypherQueryRequest>) -> Result<Response<CypherQueryResponse>, Status> {
        Err(Status::unimplemented("Not implemented"))
    }
    
    async fn list_labels(&self, _req: Request<ListLabelsRequest>) -> Result<Response<ListLabelsResponse>, Status> {
        Err(Status::unimplemented("Not implemented"))
    }
}

pub struct StorageServiceImpl {
    engine: Arc<PolyLsmEngine>,
}

impl StorageServiceImpl {
    pub fn new(engine: Arc<PolyLsmEngine>) -> Self {
        Self { engine }
    }
}

#[tonic::async_trait]
impl StorageService for StorageServiceImpl {
    async fn put_edge(
        &self,
        request: Request<PutEdgeRequest>,
    ) -> Result<Response<PutEdgeResponse>, Status> {
        let tenant_id = request
            .metadata()
            .get("tenant-id")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("default")
            .to_string();

        let req = request.into_inner();
        let source_id: i64 = req.source_id.try_into().map_err(|_| Status::invalid_argument("source_id out of range for i64"))?;
        let target_id: i64 = req.target_id.try_into().map_err(|_| Status::invalid_argument("target_id out of range for i64"))?;
        
        let empty_props = std::collections::HashMap::new();
        match self.engine.put_edge(&tenant_id, source_id.try_into().unwrap_or(0), target_id.try_into().unwrap_or(0), &empty_props) {
            Ok(_) => Ok(Response::new(PutEdgeResponse { success: true })),
            Err(e) => Err(Status::internal(format!("Failed to put edge: {}", e))),
        }
    }

    async fn get_neighbors(
        &self,
        request: Request<GetNeighborsRequest>,
    ) -> Result<Response<GetNeighborsResponse>, Status> {
        let tenant_id = request
            .metadata()
            .get("tenant-id")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("default")
            .to_string();

        let req = request.into_inner();
        let node_id: i64 = req.node_id.try_into().map_err(|_| Status::invalid_argument("node_id out of range for i64"))?;
        
        match self.engine.get_neighbors(&tenant_id, node_id.try_into().unwrap_or(0)) {
            Ok(Some(edges)) => {
                let neighbor_ids = edges.out_edges.iter().map(|&id| id as u64).collect();
                Ok(Response::new(GetNeighborsResponse { neighbor_ids }))
            }
            Ok(None) => Ok(Response::new(GetNeighborsResponse { neighbor_ids: vec![] })),
            Err(e) => Err(Status::internal(format!("Failed to get neighbors: {}", e))),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    info!("Starting sap-graph-server");

    let addr = env::var("GRAPH_SERVER_LISTEN_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:50052".to_string())
        .parse()?;
    
    let db_path = env::var("LSM_DB_PATH").unwrap_or_else(|_| "./data/lsm_db".to_string());
    
    let (engine, rx) = PolyLsmEngine::open(db_path)?;
    let engine = Arc::new(engine);

    let engine_clone = engine.clone();
    tokio::spawn(async move {
        poly_lsm_core::engine::spawn_migration_worker(engine_clone, rx).await;
    });

    let graph_service = GraphServiceImpl::new(engine.clone());
    let storage_service = StorageServiceImpl::new(engine);
    
    let (mut health_reporter, health_service) = tonic_health::server::health_reporter();
    health_reporter
        .set_serving::<GraphServiceServer<GraphServiceImpl>>()
        .await;
    health_reporter
        .set_serving::<StorageServiceServer<StorageServiceImpl>>()
        .await;

    info!("sap-graph-server listening on {}", addr);

    Server::builder()
        .add_service(health_service)
        .add_service(GraphServiceServer::new(graph_service))
        .add_service(StorageServiceServer::new(storage_service))
        .serve(addr)
        .await?;

    Ok(())
}
