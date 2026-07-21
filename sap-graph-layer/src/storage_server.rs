use tonic::{Request, Response, Status};
use std::sync::Arc;
use poly_lsm_core::{PolyLsmEngine, GraphStore};
use std::collections::HashMap;

pub mod pb {
    tonic::include_proto!("sapiola.v1");
}

use pb::storage_service_server::StorageService;
use pb::{PutEdgeRequest, PutEdgeResponse, GetNeighborsRequest, GetNeighborsResponse};

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
        let source_id: i64 = req.source_id.try_into().map_err(|_| {
            Status::invalid_argument("source_id out of range for i64")
        })?;
        let target_id: i64 = req.target_id.try_into().map_err(|_| {
            Status::invalid_argument("target_id out of range for i64")
        })?;
        
        let empty_props = HashMap::new();
        match self.engine.put_edge(&tenant_id, source_id, target_id, &empty_props) {
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
        let node_id: i64 = req.node_id.try_into().map_err(|_| {
            Status::invalid_argument("node_id out of range for i64")
        })?;
        
        match self.engine.get_neighbors(&tenant_id, node_id) {
            Ok(Some(edges)) => {
                let neighbor_ids = edges.out_edges.iter().map(|&id| id as u64).collect();
                Ok(Response::new(GetNeighborsResponse { neighbor_ids }))
            }
            Ok(None) => Ok(Response::new(GetNeighborsResponse { neighbor_ids: vec![] })),
            Err(e) => Err(Status::internal(format!("Failed to get neighbors: {}", e))),
        }
    }
}
