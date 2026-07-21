use std::collections::HashMap;
use std::fs;
use poly_lsm_core::engine::PolyLsmEngine;
use poly_lsm_core::graph_api::GraphStore;

#[test]
fn test_multi_tenant_isolation() {
    let db_path = "test_db_multi_tenant";
    let _ = fs::remove_dir_all(db_path);
    let (engine, _rx) = PolyLsmEngine::open(db_path).unwrap();

    let tenant1 = "tenant1";
    let tenant2 = "tenant2";

    let node_id = 42;

    // Both tenants use the same node_id
    engine.put_vertex(tenant1, node_id, &HashMap::from([("name".into(), "Alice".into())])).unwrap();
    engine.put_vertex(tenant2, node_id, &HashMap::from([("name".into(), "Bob".into())])).unwrap();

    let props1 = engine.get_vertex(tenant1, node_id).unwrap().unwrap();
    let props2 = engine.get_vertex(tenant2, node_id).unwrap().unwrap();

    assert_eq!(props1.get("name").unwrap(), "Alice");
    assert_eq!(props2.get("name").unwrap(), "Bob");

    // Edges isolation
    engine.put_edge(tenant1, 42, 100, &HashMap::new()).unwrap();
    engine.put_edge(tenant2, 42, 200, &HashMap::new()).unwrap();

    let n1 = engine.get_neighbors(tenant1, 42).unwrap().unwrap();
    let n2 = engine.get_neighbors(tenant2, 42).unwrap().unwrap();

    assert_eq!(n1.out_edges, vec![100]);
    assert_eq!(n2.out_edges, vec![200]);
}
