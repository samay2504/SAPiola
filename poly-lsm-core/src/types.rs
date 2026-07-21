use serde::{Deserialize, Serialize};

/// Dense integer ID space per Aster's node_id_t
pub type NodeId = i64;

/// Hybrid vertex-based layout for storing topological adjacency.
/// Keeps out-edges and in-edges sorted for fast intersection and retrieval.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Edges {
    pub out_edges: Vec<NodeId>,
    pub in_edges: Vec<NodeId>,
}

impl Edges {
    pub fn new() -> Self {
        Self {
            out_edges: Vec::new(),
            in_edges: Vec::new(),
        }
    }

    /// Add an out-edge and ensure the list remains sorted.
    pub fn add_out_edge(&mut self, target: NodeId) {
        if let Err(pos) = self.out_edges.binary_search(&target) {
            self.out_edges.insert(pos, target);
        }
    }

    /// Add an in-edge and ensure the list remains sorted.
    pub fn add_in_edge(&mut self, source: NodeId) {
        if let Err(pos) = self.in_edges.binary_search(&source) {
            self.in_edges.insert(pos, source);
        }
    }

    /// Delete an out-edge.
    pub fn remove_out_edge(&mut self, target: NodeId) {
        if let Ok(pos) = self.out_edges.binary_search(&target) {
            self.out_edges.remove(pos);
        }
    }

    /// Delete an in-edge.
    pub fn remove_in_edge(&mut self, source: NodeId) {
        if let Ok(pos) = self.in_edges.binary_search(&source) {
            self.in_edges.remove(pos);
        }
    }
}

/// A delta update to be merged into `Edges` during RocksDB compaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EdgeDelta {
    AddOutEdge(NodeId),
    AddInEdge(NodeId),
    RemoveOutEdge(NodeId),
    RemoveInEdge(NodeId),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortedRun {
    pub edges: Vec<NodeId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TieredOverflow {
    pub base_ef: Vec<u8>,
    pub runs: Vec<SortedRun>,
}

/// Represents the format of a vertex's base adjacency list stored at its vertex key.
#[derive(Debug, Clone)]
pub enum AdjacencyBlob {
    Compressed(Vec<u8>), // Standard EF (no runs yet)
    Tiered(TieredOverflow), // EF + bounded uncompressed runs
}

impl AdjacencyBlob {
    pub fn decode(bytes: &[u8]) -> anyhow::Result<Self> {
        if bytes.is_empty() {
            anyhow::bail!("Empty adjacency blob");
        }
        match bytes[0] {
            1 => Ok(Self::Compressed(bytes[1..].to_vec())),
            2 => {
                let tiered: TieredOverflow = bincode::deserialize(&bytes[1..])?;
                Ok(Self::Tiered(tiered))
            }
            _ => anyhow::bail!("Unknown adjacency blob format tag"),
        }
    }
    
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        match self {
            Self::Compressed(data) => {
                buf.push(1);
                buf.extend_from_slice(data);
            }
            Self::Tiered(tiered) => {
                buf.push(2);
                buf.extend_from_slice(&bincode::serialize(tiered).unwrap());
            }
        }
        buf
    }
}
