use vers_vecs::EliasFanoVec;

/// Encodes a SORTED, deduplicated adjacency list as Elias-Fano.
/// Do not call this on an unsorted Vec<NodeId>; sort+dedupe first.
pub fn encode_adjacency(sorted_ids: &[u64], _universe: u64) -> Vec<u8> {
    let ef = EliasFanoVec::from_slice(sorted_ids);
    bincode::serialize(&ef).expect("failed to serialize EliasFano")
}

pub fn decode_adjacency(bytes: &[u8]) -> EliasFanoVec {
    bincode::deserialize(bytes).expect("corrupt adjacency blob")
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use vers_vecs::EliasFanoVec;

    fn sorted_unique_u64() -> impl Strategy<Value = Vec<u64>> {
        prop::collection::vec(0u64..1000000, 0..1000).prop_map(|mut v| {
            v.sort_unstable();
            v.dedup();
            v
        })
    }

    proptest! {
        #[test]
        fn test_ef_roundtrip(ids in sorted_unique_u64()) {
            if ids.is_empty() {
                return Ok(());
            }
            let universe = *ids.last().unwrap() + 1;
            let encoded = encode_adjacency(&ids, universe);
            let decoded = decode_adjacency(&encoded);
            
            assert_eq!(decoded.len(), ids.len());
            for (i, &expected) in ids.iter().enumerate() {
                assert_eq!(decoded.get(i), Some(expected));
            }
        }
    }
}
