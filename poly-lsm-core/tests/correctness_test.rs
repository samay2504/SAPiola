use std::collections::HashMap;
use std::fs;

use poly_lsm_core::{GraphStore, PolyLsmEngine};

#[test]
fn test_topology_correctness() {
    let db_path = "test_db_correctness";
    let _ = fs::remove_dir_all(db_path);

    let store = PolyLsmEngine::open(db_path).expect("Failed to open store");

    // 1. Insert vertices
    let mut props = HashMap::new();
    props.insert("name".to_string(), "Alice".to_string());
    store.put_vertex(1, &props).unwrap();

    props.insert("name".to_string(), "Bob".to_string());
    store.put_vertex(2, &props).unwrap();

    props.insert("name".to_string(), "Charlie".to_string());
    store.put_vertex(3, &props).unwrap();

    // 2. Insert edges
    let eprops = HashMap::new();
    // 1 -> 2
    store.put_edge(1, 2, &eprops).unwrap();
    // 1 -> 3
    store.put_edge(1, 3, &eprops).unwrap();
    // 3 -> 2
    store.put_edge(3, 2, &eprops).unwrap();

    // 3. Verify get_neighbors topology
    // For 1, out: [2, 3], in: []
    let n1 = store.get_neighbors(1).unwrap().expect("1 should exist");
    assert_eq!(n1.out_edges, vec![2, 3]);
    assert_eq!(n1.in_edges, vec![]);

    // For 2, out: [], in: [1, 3]
    let n2 = store.get_neighbors(2).unwrap().expect("2 should exist");
    assert_eq!(n2.out_edges, vec![]);
    assert_eq!(n2.in_edges, vec![1, 3]);

    // For 3, out: [2], in: [1]
    let n3 = store.get_neighbors(3).unwrap().expect("3 should exist");
    assert_eq!(n3.out_edges, vec![2]);
    assert_eq!(n3.in_edges, vec![1]);

    // 4. Test delete edge
    store.delete_edge(1, 2).unwrap();
    let n1_after = store.get_neighbors(1).unwrap().unwrap();
    assert_eq!(n1_after.out_edges, vec![3]);

    let n2_after = store.get_neighbors(2).unwrap().unwrap();
    assert_eq!(n2_after.in_edges, vec![3]);

    // Clean up
    let _ = fs::remove_dir_all(db_path);
}
