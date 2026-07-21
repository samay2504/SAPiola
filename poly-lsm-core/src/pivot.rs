use anyhow::Result;
use fjall::{Database, Keyspace, KeyspaceCreateOptions};
use tokio::sync::mpsc;
use dashmap::DashMap;

use crate::types::NodeId;

pub const KS_DEGREE_COUNTER: &str = "degree_counter";
pub const KS_SEALED_DEGREE: &str = "sealed_degree";
pub const KS_COMPACTED_DEGREE: &str = "compacted_degree";

pub struct DegreeCounter {
    ks_degree: Keyspace,
    ks_sealed: Keyspace,
    ks_compacted: Keyspace,
    cache_degree: DashMap<Vec<u8>, u64>,
    cache_sealed: DashMap<Vec<u8>, u64>,
    cache_compacted: DashMap<Vec<u8>, u64>,
}

impl DegreeCounter {
    pub fn new(db: &Database) -> Result<Self> {
        let ks_degree = db.keyspace(KS_DEGREE_COUNTER, KeyspaceCreateOptions::default)?;
        let ks_sealed = db.keyspace(KS_SEALED_DEGREE, KeyspaceCreateOptions::default)?;
        let ks_compacted = db.keyspace(KS_COMPACTED_DEGREE, KeyspaceCreateOptions::default)?;
        Ok(Self { 
            ks_degree, 
            ks_sealed, 
            ks_compacted, 
            cache_degree: DashMap::new(),
            cache_sealed: DashMap::new(),
            cache_compacted: DashMap::new(),
        })
    }

    /// Increments the degree for a node and returns the new degree.
    pub fn increment(&self, tenant_id: &str, node_id: NodeId, batch: &mut fjall::OwnedWriteBatch) -> Result<u64> {
        let key = crate::tenant::TenantScopedKey::new(tenant_id, &node_id.to_be_bytes()).encode();
        
        let degree = {
            let mut entry = self.cache_degree.entry(key.clone()).or_insert_with(|| {
                if let Ok(Some(bytes)) = self.ks_degree.get(&key) {
                    if bytes.len() == 8 {
                        let mut buf = [0u8; 8];
                        buf.copy_from_slice(&bytes);
                        return u64::from_be_bytes(buf);
                    }
                }
                0
            });
            *entry += 1;
            *entry
        };

        batch.insert(&self.ks_degree, key, degree.to_be_bytes().to_vec());
        Ok(degree)
    }

    pub fn get_degree(&self, tenant_id: &str, node_id: NodeId) -> Result<u64> {
        let key = crate::tenant::TenantScopedKey::new(tenant_id, &node_id.to_be_bytes()).encode();
        if let Some(v) = self.cache_degree.get(&key) {
            return Ok(*v);
        }
        
        let mut val = 0;
        if let Some(bytes) = self.ks_degree.get(&key)? {
            if bytes.len() == 8 {
                let mut buf = [0u8; 8];
                buf.copy_from_slice(&bytes);
                val = u64::from_be_bytes(buf);
            }
        }
        self.cache_degree.insert(key, val);
        Ok(val)
    }

    pub fn get_sealed(&self, tenant_id: &str, node_id: NodeId) -> Result<u64> {
        let key = crate::tenant::TenantScopedKey::new(tenant_id, &node_id.to_be_bytes()).encode();
        if let Some(v) = self.cache_sealed.get(&key) {
            return Ok(*v);
        }

        let mut val = 0;
        if let Some(bytes) = self.ks_sealed.get(&key)? {
            if bytes.len() == 8 {
                let mut buf = [0u8; 8];
                buf.copy_from_slice(&bytes);
                val = u64::from_be_bytes(buf);
            }
        }
        self.cache_sealed.insert(key, val);
        Ok(val)
    }

    pub fn set_sealed(&self, tenant_id: &str, node_id: NodeId, val: u64, batch: &mut fjall::OwnedWriteBatch) -> Result<()> {
        let key = crate::tenant::TenantScopedKey::new(tenant_id, &node_id.to_be_bytes()).encode();
        self.cache_sealed.insert(key.clone(), val);
        batch.insert(&self.ks_sealed, key, val.to_be_bytes().to_vec());
        Ok(())
    }

    pub fn get_compacted(&self, tenant_id: &str, node_id: NodeId) -> Result<u64> {
        let key = crate::tenant::TenantScopedKey::new(tenant_id, &node_id.to_be_bytes()).encode();
        if let Some(v) = self.cache_compacted.get(&key) {
            return Ok(*v);
        }

        let mut val = 0;
        if let Some(bytes) = self.ks_compacted.get(&key)? {
            if bytes.len() == 8 {
                let mut buf = [0u8; 8];
                buf.copy_from_slice(&bytes);
                val = u64::from_be_bytes(buf);
            }
        }
        self.cache_compacted.insert(key, val);
        Ok(val)
    }

    pub fn set_compacted(&self, tenant_id: &str, node_id: NodeId, val: u64, batch: &mut fjall::OwnedWriteBatch) -> Result<()> {
        let key = crate::tenant::TenantScopedKey::new(tenant_id, &node_id.to_be_bytes()).encode();
        self.cache_compacted.insert(key.clone(), val);
        batch.insert(&self.ks_compacted, key, val.to_be_bytes().to_vec());
        Ok(())
    }
}

pub enum MigrationTask {
    SealRun {
        tenant_id: String,
        node_id: NodeId,
    },
}

pub type MigrationQueue = mpsc::Sender<MigrationTask>;
pub type MigrationReceiver = mpsc::Receiver<MigrationTask>;

/// Creates a new bounded migration queue.
pub fn create_migration_queue(capacity: usize) -> (MigrationQueue, MigrationReceiver) {
    mpsc::channel(capacity)
}
