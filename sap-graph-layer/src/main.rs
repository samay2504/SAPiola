use std::sync::Arc;
use tonic::{transport::Server, Request, Response, Status};
use tracing::{info, warn};
use std::env;

use poly_lsm_core::{PolyLsmEngine, GraphStore};

pub mod pb {
    tonic::include_proto!("sapiola.v1");
}
use pb::graph_service_server::{GraphService, GraphServiceServer};
use pb::storage_service_server::{StorageService, StorageServiceServer};
use pb::{
    QueryCandidatesRequest, QueryCandidatesResponse, CandidateInfo, CypherQueryRequest,
    CypherQueryResponse, ListSchemaRequest, ListSchemaResponse, PutEdgeRequest, PutEdgeResponse,
    GetNeighborsRequest, GetNeighborsResponse, PutVertexRequest, PutVertexResponse,
    GetVertexRequest, GetVertexResponse,
};

use sap_graph_layer::mapping::dsl::Catalog;
use tokio::sync::RwLock;

pub struct GraphServiceImpl {
    engine: Arc<PolyLsmEngine>,
    catalog: Arc<RwLock<Catalog>>,
}

impl GraphServiceImpl {
    pub fn new(engine: Arc<PolyLsmEngine>, catalog: Arc<RwLock<Catalog>>) -> Self {
        Self { engine, catalog }
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
    
    async fn list_schema(&self, _req: Request<ListSchemaRequest>) -> Result<Response<ListSchemaResponse>, Status> {
        let catalog = self.catalog.read().await;
        // Tenant is currently ignored in summary_for_tenant, but could be passed if needed
        let (node_labels, edge_labels) = catalog.summary_for_tenant("default");
        Ok(Response::new(ListSchemaResponse {
            node_labels,
            edge_labels,
        }))
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

    async fn put_vertex(
        &self,
        request: Request<PutVertexRequest>,
    ) -> Result<Response<PutVertexResponse>, Status> {
        let tenant_id = request
            .metadata()
            .get("tenant-id")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("default")
            .to_string();

        let req = request.into_inner();
        let node_id: i64 = req.node_id.try_into().unwrap_or(0);
        
        match self.engine.put_vertex(&tenant_id, node_id, &req.properties) {
            Ok(_) => Ok(Response::new(PutVertexResponse { success: true })),
            Err(e) => Err(Status::internal(format!("Failed to put vertex: {}", e))),
        }
    }

    async fn get_vertex(
        &self,
        request: Request<GetVertexRequest>,
    ) -> Result<Response<GetVertexResponse>, Status> {
        let tenant_id = request
            .metadata()
            .get("tenant-id")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("default")
            .to_string();

        let req = request.into_inner();
        let node_id: i64 = req.node_id.try_into().unwrap_or(0);
        
        match self.engine.get_vertex(&tenant_id, node_id) {
            Ok(Some(props)) => Ok(Response::new(GetVertexResponse { node_id: req.node_id, properties: props })),
            Ok(None) => Err(Status::not_found("Vertex not found")),
            Err(e) => Err(Status::internal(format!("Failed to get vertex: {}", e))),
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
        let node_id: i64 = req.node_id.try_into().unwrap_or(0);
        
        match self.engine.get_neighbors(&tenant_id, node_id) {
            Ok(Some(edges)) => {
                let out_edges = edges.out_edges.into_iter().map(|id| id as u64).collect();
                let in_edges = edges.in_edges.into_iter().map(|id| id as u64).collect();
                Ok(Response::new(GetNeighborsResponse { out_edges, in_edges }))
            }
            Ok(None) => Ok(Response::new(GetNeighborsResponse { out_edges: vec![], in_edges: vec![] })),
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
    
    let engine = PolyLsmEngine::open_with_worker(db_path)?;

    // Dynamic, adaptable DSL file discovery (zero hardcoding of specific dataset names)
    // Priority: 1. SAPIOLA_SCHEMA_MANIFEST (JSON manifest with embedded DSL)
    //           2. SAPIOLA_MAPPING_DSL (direct DSL file path)
    //           3. Auto-search for .dsl files in candidate directories
    let mut catalog = Catalog::new();
    let mut loaded_path: Option<String> = None;

    // Priority 1: Load from schema_manifest.json
    if let Ok(manifest_path) = env::var("SAPIOLA_SCHEMA_MANIFEST") {
        if std::path::Path::new(&manifest_path).exists() {
            match std::fs::read_to_string(&manifest_path) {
                Ok(content) => {
                    // Parse JSON and extract generated_dsl field
                    if let Ok(manifest) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(dsl_str) = manifest.get("generated_dsl").and_then(|v| v.as_str()) {
                            // Strip comment lines before loading
                            let clean_dsl: String = dsl_str
                                .lines()
                                .filter(|line| !line.trim().starts_with("//"))
                                .collect::<Vec<_>>()
                                .join("\n");
                            match Catalog::load(&clean_dsl) {
                                Ok(new_catalog) => {
                                    catalog = new_catalog;
                                    loaded_path = Some(format!("manifest:{}", manifest_path));
                                    // Log fingerprint for operator verification
                                    if let Some(fp) = manifest.get("fingerprint").and_then(|v| v.as_str()) {
                                        info!("Schema manifest fingerprint: {}", fp);
                                    }
                                }
                                Err(e) => warn!("Failed to parse DSL from manifest {}: {}", manifest_path, e),
                            }
                        }
                    }
                }
                Err(e) => warn!("Failed to read manifest {}: {}", manifest_path, e),
            }
        }
    }

    // Priority 2: Direct DSL file path
    if loaded_path.is_none() {
        if let Ok(dsl_env_path) = env::var("SAPIOLA_MAPPING_DSL") {
            if std::path::Path::new(&dsl_env_path).exists() {
                if let Ok(()) = catalog.reload(&dsl_env_path) {
                    loaded_path = Some(dsl_env_path);
                }
            }
        }
    }

    // Priority 3: Auto-search for .dsl files
    if loaded_path.is_none() {
        let search_dirs = ["tools", "config", ".", "..", "../tools"];
        'dir_loop: for dir in &search_dirs {
            let path = std::path::Path::new(dir);
            if let Ok(entries) = std::fs::read_dir(path) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_file() && p.extension().map_or(false, |ext| ext == "dsl") {
                        let p_str = p.to_string_lossy().to_string();
                        if let Ok(()) = catalog.reload(&p_str) {
                            loaded_path = Some(p_str);
                            break 'dir_loop;
                        }
                    } else if p.is_dir() {
                        if let Ok(sub_entries) = std::fs::read_dir(&p) {
                            for sub_entry in sub_entries.flatten() {
                                let sub_p = sub_entry.path();
                                if sub_p.is_file() && sub_p.extension().map_or(false, |ext| ext == "dsl") {
                                    let sub_p_str = sub_p.to_string_lossy().to_string();
                                    if let Ok(()) = catalog.reload(&sub_p_str) {
                                        loaded_path = Some(sub_p_str);
                                        break 'dir_loop;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    match loaded_path {
        Some(path) => info!("Successfully loaded dynamic SAP schema DSL from {}", path),
        None => warn!("No SAP schema .dsl file found in SAPIOLA_SCHEMA_MANIFEST, SAPIOLA_MAPPING_DSL, or search directories. ListSchema will be empty until populated."),
    }
    let catalog = Arc::new(RwLock::new(catalog));

    let graph_service = GraphServiceImpl::new(engine.clone(), catalog.clone());
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
