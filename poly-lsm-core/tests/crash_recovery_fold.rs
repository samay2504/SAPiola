use poly_lsm_core::{GraphStore, PolyLsmEngine};
use std::collections::HashMap;
use std::process::Command;
use std::sync::Arc;

fn main() {
    let db_path = "test_db_crash_fold";
    let tenant = "tenant1";
    let node_id = 1;

    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "child" {
        let (engine, rx) = PolyLsmEngine::open(db_path).unwrap();
        let store = Arc::new(engine);
        let worker_store = Arc::clone(&store);
        
        tokio::runtime::Runtime::new().unwrap().spawn(async move {
            poly_lsm_core::engine::spawn_migration_worker(worker_store, rx).await;
        });

        store.put_vertex(tenant, node_id, &HashMap::new()).unwrap();

        // Write edges until fold is triggered (12000 uncompacted runs -> degree ~13000)
        let mut dst = 2;
        let props = HashMap::new();
        loop {
            store.put_edge(tenant, node_id, dst, &props).unwrap();
            dst += 1;
            
            // We just spin forever, the trap in `synchronous_fold` will abort the process.
            if dst % 1000 == 0 {
                println!("CHILD: Reached dst={}", dst);
            }
        }
    } else {
        let _ = std::fs::remove_dir_all(db_path);

        println!("PARENT: Spawning child process...");
        let exe = std::env::current_exe().unwrap();
        let mut child = Command::new(exe)
            .arg("child")
            .env("CRASH_DURING_FOLD", "1")
            .spawn()
            .unwrap();

        let status = child.wait().unwrap();
        println!("PARENT: Child exited with status: {:?}", status);

        // Re-open engine
        println!("PARENT: Re-opening DB to verify crash consistency mid-fold...");
        let (engine, _rx) = PolyLsmEngine::open(db_path).unwrap();
        
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
        let mut expected = 2;
        let mut missing = Vec::new();
        for d in dests {
            if d != expected {
                missing.push(expected);
                expected = d + 1;
            } else {
                expected += 1;
            }
        }
        
        let last_edge = if expected > 2 { expected - 1 } else { 0 };
        println!("PARENT: Found contiguous edges up to {}. Missing edges inside that range: {:?}", last_edge, missing);
        
        assert!(missing.is_empty(), "Mid-fold crash resulted in dropped edges!");
        assert_eq!(degree as usize, neighbors.out_edges.len(), "Degree counter must exactly match the returned out-edges!");
        assert!(sealed >= 12000, "Expected at least 12000 sealed to trigger fold!");
        assert_eq!(compacted, 0, "Compacted must still be 0 since the fold batch was aborted before commit!");

        println!("PARENT: CRASH RECOVERY SUCCESS!");
        let _ = std::fs::remove_dir_all(db_path);
    }
}
