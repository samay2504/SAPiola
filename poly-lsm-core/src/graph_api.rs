use std::collections::HashMap;
use anyhow::Result;

use crate::types::{Edges, NodeId};

/// The storage backend interface for the Poly-LSM graph database.
pub trait GraphStore {
    /// Inserts or updates a vertex and its properties.
    /// This writes to the `vprop_val` column family.
    fn put_vertex(&self, tenant_id: &str, id: NodeId, props: &HashMap<String, String>) -> Result<()>;

    /// Inserts an edge between two vertices and its properties.
    /// This uses a merge operator to write an edge delta to the `topology` column family,
    /// and writes the properties to the `eprop_val` column family.
    fn put_edge(&self, tenant_id: &str, src: NodeId, dst: NodeId, props: &HashMap<String, String>) -> Result<()>;

    /// Deletes an edge between two vertices.
    /// This uses a merge operator to write an edge deletion delta to the `topology` column family.
    fn delete_edge(&self, tenant_id: &str, src: NodeId, dst: NodeId) -> Result<()>;

    /// Retrieves the topological adjacency lists (in-edges and out-edges) for a given vertex.
    /// This reads from the `topology` column family.
    fn get_neighbors(&self, tenant_id: &str, id: NodeId) -> Result<Option<Edges>>;

    /// Retrieves the properties for a given vertex.
    /// This reads from the `vprop_val` column family.
    fn get_vertex(&self, tenant_id: &str, id: NodeId) -> Result<Option<HashMap<String, String>>>;
    
    /// Scans all vertices for a given tenant, returning their ID and properties.
    /// Used for naive full-text candidate discovery in V1.
    fn scan_vertices(&self, tenant_id: &str) -> Result<Vec<(NodeId, HashMap<String, String>)>>;
    
    /// Retrieves the properties for a given edge.
    /// This reads from the `eprop_val` column family.
    fn get_edge(&self, tenant_id: &str, src: NodeId, dst: NodeId) -> Result<Option<HashMap<String, String>>>;

    /// Flushes all pending writes to disk (fsync).
    fn flush(&self) -> Result<()>;
}
