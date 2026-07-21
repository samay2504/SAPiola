use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::types::NodeId;

const PATH_SEP: u8 = 0x1f;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PointerPathEntry {
    pub node_id: NodeId,
    pub path: Vec<String>,
    pub sap_key: String,
}

impl PointerPathEntry {
    pub fn encode_prefix(path: &[String]) -> Vec<u8> {
        let mut encoded = Vec::new();
        for segment in path {
            if !encoded.is_empty() {
                encoded.push(PATH_SEP);
            }
            encoded.extend_from_slice(segment.as_bytes());
        }
        if !encoded.is_empty() {
            encoded.push(PATH_SEP);
        }
        encoded
    }

    pub fn encode_key(path: &[String], node_id: NodeId) -> Vec<u8> {
        let mut key = Self::encode_prefix(path);
        key.extend_from_slice(&node_id.to_be_bytes());
        key
    }
}

pub trait PointerIndexStore {
    fn put_pointer_path(&self, tenant_id: &str, entry: &PointerPathEntry) -> Result<()>;

    fn get_pointer_paths_by_prefix(&self, tenant_id: &str, prefix: &[String]) -> Result<Vec<PointerPathEntry>>;
}

#[cfg(test)]
mod tests {
    use super::{PointerPathEntry, PATH_SEP};

    #[test]
    fn encodes_prefix_with_separator_boundaries() {
        let prefix = vec!["Company".to_string(), "BU".to_string(), "Plant".to_string()];
        let encoded = PointerPathEntry::encode_prefix(&prefix);

        assert_eq!(encoded, b"Company\x1fBU\x1fPlant\x1f".to_vec());
        assert_eq!(PATH_SEP, 0x1f);
    }

    #[test]
    fn appends_node_id_to_encoded_key() {
        let prefix = vec!["Company".to_string(), "BU".to_string()];
        let encoded = PointerPathEntry::encode_key(&prefix, 42);

        assert!(encoded.starts_with(b"Company\x1fBU\x1f"));
        assert_eq!(&encoded[encoded.len() - 8..], &42i64.to_be_bytes());
    }
}