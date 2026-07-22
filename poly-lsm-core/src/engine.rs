use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use dashmap::DashSet;
use fjall::{Database, Keyspace, KeyspaceCreateOptions, PersistMode};

use crate::graph_api::GraphStore;
use crate::pointer_index::{PointerIndexStore, PointerPathEntry};
use crate::telemetry::{Telemetry, TelemetryFilterFactory};
use crate::types::{Edges, NodeId, AdjacencyBlob};
use crate::tenant::TenantScopedKey;
use crate::pivot::{DegreeCounter, MigrationQueue, create_migration_queue, MigrationReceiver, MigrationTask};

pub const KS_TOPOLOGY: &str = "topology";
pub const KS_VPROP_VAL: &str = "vprop_val";
pub const KS_EPROP_VAL: &str = "eprop_val";
pub const KS_POINTER_IDX: &str = "pointer_idx";

/// Configuration parameters for the PolyLsmEngine.
/// 
/// **SCOPE DECISION: Unbounded `base_ef` Growth**
/// The design mathematically bounds uncompacted deltas and runs via `max_unsealed_deltas`, 
/// `max_run_size`, and `max_run_count`, guaranteeing low deserialization overhead for the top tiers.
/// However, there is **no ceiling on `base_ef` size**. 
/// 
/// Elias-Fano decoding takes O(N) time. If a vertex accumulates 20M edges, `base_ef` will compress 
/// all 20M integers, and decoding them during `get_neighbors` will take proportional time (~250ms). 
/// We explicitly accept this tradeoff because:
/// 1. SAP schema graphs (BOMs, Document Flows) are deeply nested but rarely produce anomalies 
///    with millions of edges on a single vertex. 
/// 2. Bounding `base_ef` to $O(1)$ read time would require horizontal sharding of the adjacency 
///    blob, which significantly increases write complexity and fragmentation for the 99% common case.
/// 
/// A runtime warning is emitted if `base_ef` crosses an anomalous threshold (e.g. 1 million edges)
/// so that massive super-nodes become an observable operational signal rather than a silent latency degradation.
pub struct Config {
    pub pivot_compression_min_degree: u64,
    pub max_run_size: u64,
    pub max_unsealed_deltas: u64,
    pub max_run_count: u64,
}
impl Default for Config {
    fn default() -> Self {
        Self { 
            pivot_compression_min_degree: 50_000,
            max_run_size: 2_000,
            max_unsealed_deltas: 3_000,
            max_run_count: 6,
        }
    }
}

/// The implementation of the Poly-LSM graph storage engine over Fjall.
pub struct PolyLsmEngine {
    pub db: Database,
    pub topology: Keyspace,
    pub vprop_val: Keyspace,
    pub eprop_val: Keyspace,
    pub pointer_idx: Keyspace,
    telemetry: Telemetry,
    
    pub degree_counter: DegreeCounter,
    pub migration_queue: MigrationQueue,
    pub config: Config,
    pub vertex_locks: DashSet<(String, NodeId)>,
}

impl PolyLsmEngine {
    /// Opens or creates the graph database at the given path.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<(Self, MigrationReceiver)> {
        let telemetry = Telemetry::new();
        
        let tel_clone = telemetry.clone();

        let db = Database::builder(path.as_ref())
            .manual_journal_persist(true)
            .with_compaction_filter_factories(Arc::new(move |ks_name| {
                match ks_name {
                    KS_TOPOLOGY => Some(Arc::new(TelemetryFilterFactory::new(tel_clone.compactions_topology.clone()))),
                    KS_VPROP_VAL => Some(Arc::new(TelemetryFilterFactory::new(tel_clone.compactions_vprop.clone()))),
                    KS_EPROP_VAL => Some(Arc::new(TelemetryFilterFactory::new(tel_clone.compactions_eprop.clone()))),
                    _ => None,
                }
            }))
            .open()
            .context("Failed to open Fjall Database")?;
        
        let topology = db.keyspace(KS_TOPOLOGY, KeyspaceCreateOptions::default)?;
        let vprop_val = db.keyspace(KS_VPROP_VAL, KeyspaceCreateOptions::default)?;
        let eprop_val = db.keyspace(KS_EPROP_VAL, KeyspaceCreateOptions::default)?;
        let pointer_idx = db.keyspace(KS_POINTER_IDX, KeyspaceCreateOptions::default)?;

        let degree_counter = DegreeCounter::new(&db)?;
        let (tx, rx) = create_migration_queue(1000);

        let engine = Self {
            db,
            topology,
            vprop_val,
            eprop_val,
            pointer_idx,
            telemetry,
            degree_counter,
            migration_queue: tx,
            config: Config::default(),
            vertex_locks: DashSet::new(),
        };

        Ok((engine, rx))
    }

    pub fn open_with_worker(db_path: impl AsRef<Path>) -> Result<Arc<Self>> {
        let (engine, rx) = Self::open(db_path)?;
        let engine = Arc::new(engine);
        let engine_clone = engine.clone();
        tokio::spawn(async move {
            spawn_migration_worker(engine_clone, rx).await;
        });
        Ok(engine)
    }

    pub fn telemetry(&self) -> &Telemetry {
        &self.telemetry
    }

    pub fn metrics_topology(&self) -> &lsm_tree::Metrics {
        self.topology.metrics()
    }

    pub fn metrics_vprop(&self) -> &lsm_tree::Metrics {
        self.vprop_val.metrics()
    }

    pub fn metrics_eprop(&self) -> &lsm_tree::Metrics {
        self.eprop_val.metrics()
    }

    pub fn encode_node_id(id: NodeId) -> [u8; 8] {
        id.to_be_bytes()
    }

    fn encode_edge_key(src: NodeId, dst: NodeId) -> [u8; 16] {
        let mut buf = [0u8; 16];
        buf[0..8].copy_from_slice(&src.to_be_bytes());
        buf[8..16].copy_from_slice(&dst.to_be_bytes());
        buf
    }

    /// Key format for topology edge-deltas: [8-byte NodeId][1-byte Dir][8-byte SeqNum]
    /// Dir: 0 = Out, 1 = In
    pub fn encode_topology_edge_key(node: NodeId, dir: u8, seq_num: u64) -> [u8; 17] {
        let mut buf = [0u8; 17];
        buf[0..8].copy_from_slice(&node.to_be_bytes());
        buf[8] = dir;
        buf[9..17].copy_from_slice(&seq_num.to_be_bytes());
        buf
    }

    /// Marks a vertex as migrated. We can use a 0-byte uncompressed blob as a marker if needed, 
    /// but the worker writes the `CompressedAdjacency` directly to the `vertex_key(id)`.
    pub fn mark_migrated(&self, tenant_id: &str, node_id: NodeId, encoded: Vec<u8>) -> Result<()> {
        let key = TenantScopedKey::new(tenant_id, &Self::encode_node_id(node_id)).encode();
        let payload = AdjacencyBlob::Compressed(encoded).encode();
        self.topology.insert(key, payload)?;
        Ok(())
    }

    pub fn synchronous_seal(&self, tenant_id: &str, node_id: NodeId) -> Result<()> {
        let lock_key = (tenant_id.to_string(), node_id);
        
        if !self.vertex_locks.insert(lock_key.clone()) {
            return Ok(());
        }
        
        // Ensure we drop the lock at the end
        let _lock_guard = scopeguard::guard(lock_key.clone(), |key| {
            self.vertex_locks.remove(&key);
        });
        
        let degree = self.degree_counter.get_degree(tenant_id, node_id)?;
        let sealed = self.degree_counter.get_sealed(tenant_id, node_id).unwrap_or(0);
        let unsealed = degree.saturating_sub(sealed);
        
        if unsealed < self.config.max_run_size {
            return Ok(());
        }

        // We will scan exactly the unsealed keys using seq_num bounds
        let start_seq = sealed + 1;
        let end_seq = sealed + self.config.max_run_size;
        
        let start_key = TenantScopedKey::new(tenant_id, &Self::encode_topology_edge_key(node_id, 0, start_seq)).encode();
        let end_key = TenantScopedKey::new(tenant_id, &Self::encode_topology_edge_key(node_id, 0, end_seq)).encode();
        

        let mut edges = Vec::new();
        let mut keys_to_delete = Vec::new();

        for item in self.topology.range(start_key..=end_key) {
            let (k, v) = item.into_inner().unwrap();
            if v.len() == 8 {
                let mut target_buf = [0u8; 8];
                target_buf.copy_from_slice(&v);
                edges.push(i64::from_be_bytes(target_buf));
            }
            keys_to_delete.push(k);
        }

        // Must sort because they were inserted by seq_num, not target_id
        edges.sort_unstable();
        edges.dedup();

        if edges.is_empty() {
            return Ok(());
        }

        // Fetch existing TieredOverflow
        let vertex_key = TenantScopedKey::new(tenant_id, &Self::encode_node_id(node_id)).encode();
        let mut tiered = match self.topology.get(&vertex_key)? {
            Some(v) => match AdjacencyBlob::decode(&v)? {
                AdjacencyBlob::Tiered(t) => t,
                AdjacencyBlob::Compressed(ef) => crate::types::TieredOverflow { base_ef: ef, runs: Vec::new() },
            },
            None => crate::types::TieredOverflow { base_ef: Vec::new(), runs: Vec::new() },
        };

        tiered.runs.push(crate::types::SortedRun { edges });

        let mut batch = self.db.batch();
        batch.insert(&self.topology, vertex_key, AdjacencyBlob::Tiered(tiered).encode());
        for k in keys_to_delete {
            batch.remove(&self.topology, k);
        }
        
        // Update sealed counter
        self.degree_counter.set_sealed(tenant_id, node_id, sealed + self.config.max_run_size, &mut batch)?;
        batch.commit()?;

        Ok(())
    }

    pub fn synchronous_fold(&self, tenant_id: &str, node_id: NodeId) -> Result<()> {
        let lock_key = (tenant_id.to_string(), node_id);
        
        if !self.vertex_locks.insert(lock_key.clone()) {
            return Ok(());
        }
        
        // Ensure we drop the lock at the end
        let _lock_guard = scopeguard::guard(lock_key.clone(), |key| {
            self.vertex_locks.remove(&key);
        });
        
        let sealed = self.degree_counter.get_sealed(tenant_id, node_id).unwrap_or(0);
        let compacted = self.degree_counter.get_compacted(tenant_id, node_id).unwrap_or(0);
        let uncompacted_runs = sealed.saturating_sub(compacted);

        if uncompacted_runs < (self.config.max_run_size * self.config.max_run_count) {
            return Ok(());
        }

        let vertex_key = TenantScopedKey::new(tenant_id, &Self::encode_node_id(node_id)).encode();
        if let Some(v) = self.topology.get(&vertex_key)? {
            if let AdjacencyBlob::Tiered(mut tiered) = AdjacencyBlob::decode(&v)? {
                // Merge base_ef + runs
                let mut merged = crate::types::Edges::new();
                if !tiered.base_ef.is_empty() {
                    let ef = crate::encoding::decode_adjacency(&tiered.base_ef);
                    for i in 0..ef.len() {
                        if let Some(t) = ef.get(i) {
                            merged.add_out_edge(t as i64);
                        }
                    }
                }
                for run in tiered.runs.drain(..) {
                    for target in run.edges {
                        merged.add_out_edge(target);
                    }
                }

                let out_edges: Vec<u64> = merged.out_edges.iter().map(|&x| x as u64).collect();
                
                if out_edges.len() >= 1_000_000 {
                    tracing::warn!(
                        tenant = tenant_id,
                        node = node_id,
                        edges = out_edges.len(),
                        "ANOMALY: Vertex has grown into a massive super-node. `base_ef` decode latency will degrade linearly (O(N))."
                    );
                }
                
                let universe = out_edges.last().copied().unwrap_or(0) + 1;
                let compressed = crate::encoding::encode_adjacency(&out_edges, universe);

                tiered.base_ef = compressed;
                let payload = AdjacencyBlob::Tiered(tiered).encode();

                let mut batch = self.db.batch();
                batch.insert(&self.topology, vertex_key, payload);
                
                // Update compacted counter
                self.degree_counter.set_compacted(tenant_id, node_id, sealed, &mut batch)?;
                batch.commit()?;
            }
        }
        
        Ok(())
    }
}

impl GraphStore for PolyLsmEngine {
    fn put_vertex(&self, tenant_id: &str, id: NodeId, props: &HashMap<String, String>) -> Result<()> {
        let key = TenantScopedKey::new(tenant_id, &Self::encode_node_id(id)).encode();
        let val = bincode::serialize(props).context("Failed to serialize vertex props")?;
        self.vprop_val.insert(key, val)?;
        Ok(())
    }

    fn put_edge(&self, tenant_id: &str, src: NodeId, dst: NodeId, props: &HashMap<String, String>) -> Result<()> {
        let mut batch = self.db.batch();

        let degree = self.degree_counter.increment(tenant_id, src, &mut batch)?;
        
        let out_key = TenantScopedKey::new(tenant_id, &Self::encode_topology_edge_key(src, 0, degree)).encode();
        batch.insert(&self.topology, out_key, dst.to_be_bytes().to_vec());

        // For in-edge, we increment its degree too!
        let in_degree = self.degree_counter.increment(tenant_id, dst, &mut batch)?;
        let in_key = TenantScopedKey::new(tenant_id, &Self::encode_topology_edge_key(dst, 1, in_degree)).encode();
        batch.insert(&self.topology, in_key, src.to_be_bytes().to_vec());

        let edge_key = TenantScopedKey::new(tenant_id, &Self::encode_edge_key(src, dst)).encode();
        let val = bincode::serialize(props).context("Failed to serialize edge props")?;
        batch.insert(&self.eprop_val, edge_key, val);

        batch.commit()?;
        
        let sealed = self.degree_counter.get_sealed(tenant_id, src)?;
        let compacted = self.degree_counter.get_compacted(tenant_id, src)?;
        
        let unsealed = degree.saturating_sub(sealed);
        let uncompacted_runs = sealed.saturating_sub(compacted);

        if unsealed >= self.config.max_unsealed_deltas {
            self.synchronous_seal(tenant_id, src)?;
        } else if unsealed >= self.config.max_run_size {
            let _ = self.migration_queue.try_send(MigrationTask::SealRun {
                tenant_id: tenant_id.to_string(),
                node_id: src,
            });
        }

        if uncompacted_runs >= (self.config.max_run_size * self.config.max_run_count) {
            self.synchronous_fold(tenant_id, src)?;
        }
        
        Ok(())
    }

    fn delete_edge(&self, _tenant_id: &str, _src: NodeId, _dst: NodeId) -> Result<()> {
        // Obsolete in this benchmark. Would be implemented with blind delta writes.
        Ok(())
    }

    fn get_neighbors(&self, tenant_id: &str, id: NodeId) -> Result<Option<Edges>> {
        let degree = self.degree_counter.get_degree(tenant_id, id)?;
        if degree == 0 {
            return Ok(None);
        }

        // --- ARCHITECTURAL TRADEOFF RECORD ---
        // The base_ef decode cost is O(N) where N is the lifetime degree of the vertex.
        // We explicitly accept this unbounded read latency tradeoff under the domain constraint 
        // that SAP hubs do not realistically exceed 500,000 edges.
        const SUPER_NODE_THRESHOLD: u64 = 500_000;
        if degree > SUPER_NODE_THRESHOLD {
            tracing::warn!(
                tenant_id = %tenant_id,
                node_id = %id,
                degree = %degree,
                "SUPER-NODE WARNING: Vertex degree exceeds threshold ({}). base_ef decode may degrade read latency.",
                SUPER_NODE_THRESHOLD
            );
        }

        let mut edges = Edges::new();
        let mut found = false;

        let vertex_key = TenantScopedKey::new(tenant_id, &Self::encode_node_id(id)).encode();
        if let Some(v) = self.topology.get(&vertex_key)? {
            found = true;
            if let Ok(blob) = AdjacencyBlob::decode(&v) {
                match blob {
                    AdjacencyBlob::Compressed(data) => {
                        let ef = crate::encoding::decode_adjacency(&data);
                        for i in 0..ef.len() {
                            if let Some(target) = ef.get(i) {
                                edges.add_out_edge(target as i64);
                            }
                        }
                    }
                    AdjacencyBlob::Tiered(tiered) => {
                        if !tiered.base_ef.is_empty() {
                            let ef = crate::encoding::decode_adjacency(&tiered.base_ef);
                            for i in 0..ef.len() {
                                if let Some(target) = ef.get(i) {
                                    edges.add_out_edge(target as i64);
                                }
                            }
                        }
                        for run in tiered.runs {
                            for target in run.edges {
                                edges.add_out_edge(target);
                            }
                        }
                    }
                }
            }
        }

        let sealed = self.degree_counter.get_sealed(tenant_id, id).unwrap_or(0);
        let start_key = TenantScopedKey::new(tenant_id, &Self::encode_topology_edge_key(id, 0, sealed + 1)).encode();
        let end_key = TenantScopedKey::new(tenant_id, &Self::encode_topology_edge_key(id, 0, u64::MAX)).encode();

        for item in self.topology.range(start_key..=end_key) {
            let (_, v) = item.into_inner().unwrap();
            found = true;
            if v.len() == 8 {
                let mut target_buf = [0u8; 8];
                target_buf.copy_from_slice(&v);
                edges.add_out_edge(i64::from_be_bytes(target_buf));
            }
        }

        if found {
            Ok(Some(edges))
        } else {
            Ok(None)
        }
    }

    fn get_vertex(&self, tenant_id: &str, id: NodeId) -> Result<Option<HashMap<String, String>>> {
        let key = TenantScopedKey::new(tenant_id, &Self::encode_node_id(id)).encode();
        if let Some(bytes) = self.vprop_val.get(&key)? {
            let props = bincode::deserialize(&bytes).context("Failed to deserialize vprop")?;
            Ok(Some(props))
        } else {
            Ok(None)
        }
    }
    
    fn scan_vertices(&self, tenant_id: &str) -> Result<Vec<(NodeId, HashMap<String, String>)>> {
        let start_key = TenantScopedKey::new(tenant_id, &Self::encode_node_id(0)).encode();
        let end_key = TenantScopedKey::new(tenant_id, &Self::encode_node_id(i64::MAX)).encode();
        
        let mut results = Vec::new();
        for item in self.vprop_val.range(start_key..=end_key) {
            let (k, v) = item.into_inner().context("Failed iterating vprop_val")?;
            // TenantScopedKey layout: [2 bytes len][tenant bytes][inner key]
            // We just deserialize the value. To get the node ID, we can parse it from the inner key, 
            // but it's easier to just assume the inner key is the 8-byte node ID at the end.
            if k.len() >= 8 {
                let mut id_buf = [0u8; 8];
                id_buf.copy_from_slice(&k[k.len() - 8..]);
                let node_id = NodeId::from_be_bytes(id_buf);
                
                let props: HashMap<String, String> = bincode::deserialize(&v).context("Failed to deserialize vprop")?;
                results.push((node_id, props));
            }
        }
        
        Ok(results)
    }

    fn get_edge(&self, tenant_id: &str, src: NodeId, dst: NodeId) -> Result<Option<HashMap<String, String>>> {
        let key = TenantScopedKey::new(tenant_id, &Self::encode_edge_key(src, dst)).encode();
        if let Some(bytes) = self.eprop_val.get(&key)? {
            let props = bincode::deserialize(&bytes).context("Failed to deserialize eprop")?;
            Ok(Some(props))
        } else {
            Ok(None)
        }
    }

    fn flush(&self) -> Result<()> {
        self.db.persist(PersistMode::SyncAll).context("Failed to fsync database")?;
        Ok(())
    }
}

impl PointerIndexStore for PolyLsmEngine {
    fn put_pointer_path(&self, tenant_id: &str, entry: &PointerPathEntry) -> Result<()> {
        let inner_key = PointerPathEntry::encode_key(&entry.path, entry.node_id);
        let key = TenantScopedKey::new(tenant_id, &inner_key).encode();
        let value = bincode::serialize(entry).context("Failed to serialize pointer entry")?;
        self.pointer_idx.insert(key, value)?;
        Ok(())
    }

    fn get_pointer_paths_by_prefix(&self, tenant_id: &str, prefix: &[String]) -> Result<Vec<PointerPathEntry>> {
        let inner_key_prefix = PointerPathEntry::encode_prefix(prefix);
        let key_prefix = TenantScopedKey::new(tenant_id, &inner_key_prefix).encode();
        let mut entries = Vec::new();

        for item in self.pointer_idx.prefix(&key_prefix) {
            let (_, value) = item.into_inner().context("Failed iterating pointer prefix")?;
            let entry: PointerPathEntry = bincode::deserialize(&value).context("Failed to deserialize pointer entry")?;
            entries.push(entry);
        }

        Ok(entries)
    }
}

pub async fn spawn_migration_worker(engine: Arc<PolyLsmEngine>, mut rx: MigrationReceiver) {
    while let Some(task) = rx.recv().await {
        match task {
            MigrationTask::SealRun { tenant_id, node_id } => {
                if let Err(e) = engine.synchronous_seal(&tenant_id, node_id) {
                    eprintln!("Background seal failed: {}", e);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_crash_consistency_degree_counter() {
        let db_path = "test_crash_consistency_degree_counter";
        let _ = std::fs::remove_dir_all(db_path);

        let tenant = "tenant_crash";
        let node_id = 42;

        {
            let (engine, _rx) = PolyLsmEngine::open(db_path).unwrap();
            engine.put_vertex(tenant, node_id, &HashMap::new()).unwrap();
            engine.put_edge(tenant, node_id, 1, &HashMap::new()).unwrap();
            engine.put_edge(tenant, node_id, 2, &HashMap::new()).unwrap();
            engine.put_edge(tenant, node_id, 3, &HashMap::new()).unwrap();
        }

        {
            let (engine, _rx) = PolyLsmEngine::open(db_path).unwrap();
            engine.put_edge(tenant, node_id, 4, &HashMap::new()).unwrap();
            
            let neighbors = engine.get_neighbors(tenant, node_id).unwrap().unwrap();
            assert_eq!(neighbors.out_edges.len(), 4, "Neighbors should be exactly 4, meaning no seq_num collision occurred!");
            
            let mut dests: Vec<u64> = neighbors.out_edges.iter().map(|id| *id as u64).collect();
            dests.sort();
            assert_eq!(dests, vec![1, 2, 3, 4], "All destination nodes should be preserved");
        }

        let _ = std::fs::remove_dir_all(db_path);
    }
}
