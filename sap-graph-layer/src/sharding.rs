use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Simple partitioner that uses `hash(source) % N` routing.
pub struct ShardRouter {
    num_shards: usize,
}

impl ShardRouter {
    pub fn new(num_shards: usize) -> Self {
        assert!(num_shards > 0, "num_shards must be > 0");
        Self { num_shards }
    }

    pub fn route_edge(&self, source_id: u64, _target_id: u64) -> usize {
        let mut hasher = DefaultHasher::new();
        source_id.hash(&mut hasher);
        (hasher.finish() as usize) % self.num_shards
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_routing() {
        let router = ShardRouter::new(4);
        let s1 = router.route_edge(100, 200);
        let s2 = router.route_edge(100, 300);
        // All edges originating from the same source node must route to the same shard.
        assert_eq!(s1, s2);
    }
}
