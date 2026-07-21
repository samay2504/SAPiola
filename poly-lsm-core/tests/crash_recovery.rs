use poly_lsm_core::{GraphStore, PolyLsmEngine};
use std::collections::HashMap;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

fn main() {
    let db_path = "test_db_crash_sigkill";
    let tenant = "tenant1";
    let node_id = 1;

    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "child" {
        // Child process: write to node_id in a tight loop and abruptly abort
        let (engine, rx) = PolyLsmEngine::open(db_path).unwrap();
        let store = Arc::new(engine);
        let worker_store = Arc::clone(&store);
        
        tokio::runtime::Runtime::new().unwrap().spawn(async move {
            poly_lsm_core::engine::spawn_migration_worker(worker_store, rx).await;
        });

        store.put_vertex(tenant, node_id, &HashMap::new()).unwrap();

        // Write edges until we get killed
        let mut dst = 2;
        let props = HashMap::new();
        loop {
            store.put_edge(tenant, node_id, dst, &props).unwrap();
            dst += 1;
            
            // Abort aggressively mid-migration around 5500 edges
            // 5500 edges = 1 sealed run (2000), maybe 1 base EF fold (4000), 1500 deltas.
            if dst > 5500 {
                // Abort abruptly (SIGKILL equivalent)
                println!("CHILD: Aborting at dst={}", dst);
                std::process::abort();
            }
        }
    } else {
        // Parent process
        let _ = std::fs::remove_dir_all(db_path);

        println!("PARENT: Spawning child process...");
        let exe = std::env::current_exe().unwrap();
        let mut child = Command::new(exe)
            .arg("child")
            .spawn()
            .unwrap();

        let status = child.wait().unwrap();
        println!("PARENT: Child exited with status: {:?}", status);

        // Re-open engine
        println!("PARENT: Re-opening DB to verify crash consistency...");
        let (engine, _rx) = PolyLsmEngine::open(db_path).unwrap();
        
        // Let's see what we get!
        let neighbors = engine.get_neighbors(tenant, node_id).unwrap().unwrap();
        let degree = engine.degree_counter.get_degree(tenant, node_id).unwrap();
        let sealed = engine.degree_counter.get_sealed(tenant, node_id).unwrap();
        let compacted = engine.degree_counter.get_compacted(tenant, node_id).unwrap();

        println!("PARENT: Recovered Degree: {}", degree);
        println!("PARENT: Recovered Sealed: {}", sealed);
        println!("PARENT: Recovered Compacted: {}", compacted);
        println!("PARENT: Recovered Neighbors Count: {}", neighbors.out_edges.len());

        let mut dests: Vec<u64> = neighbors.out_edges.iter().map(|x| *x as u64).collect();
        dests.sort();
        let mut expected = 1;
        let mut missing = Vec::new();
        for d in dests {
            if d != expected {
                missing.push(expected);
                expected = d + 1;
            } else {
                expected += 1;
            }
        }
        println!("PARENT: Missing edges: {:?}", missing);
        
        // Let's explicitly check if edge 5494 exists in topology
        let out_key = poly_lsm_core::tenant::TenantScopedKey::new(tenant, &poly_lsm_core::engine::PolyLsmEngine::encode_topology_edge_key(node_id, 0, degree)).encode();
        let val = engine.topology.get(&out_key).unwrap();
        println!("PARENT: topology.get(seq_num={}) = {:?}", degree, val.is_some());
        
        let start_key = poly_lsm_core::tenant::TenantScopedKey::new(tenant, &poly_lsm_core::engine::PolyLsmEngine::encode_topology_edge_key(node_id, 0, sealed + 1)).encode();
        let end_key = poly_lsm_core::tenant::TenantScopedKey::new(tenant, &poly_lsm_core::engine::PolyLsmEngine::encode_topology_edge_key(node_id, 0, u64::MAX)).encode();
        let mut count = 0;
        let mut last_seen = 0;
        for item in engine.topology.range(start_key..=end_key) {
            let (k, v) = item.into_inner().unwrap();
            let mut seq_buf = [0u8; 8];
            seq_buf.copy_from_slice(&k[k.len()-8..]);
            let seq = u64::from_be_bytes(seq_buf);
            last_seen = seq;
            count += 1;
        }
        println!("PARENT: Range scan yielded {} items. Last seen seq_num: {}", count, last_seen);
        
        assert_eq!(degree as usize, neighbors.out_edges.len(), "Degree counter must exactly match the returned edges!");
        
        // Assert we actually hit mid-migration!
        assert!(sealed > 0, "Expected at least one sealed run to test mid-migration!");

        println!("PARENT: CRASH RECOVERY SUCCESS!");
        let _ = std::fs::remove_dir_all(db_path);
    }
}
