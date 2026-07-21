use std::collections::HashMap;
use std::fs;
use std::time::Instant;

use poly_lsm_core::{GraphStore, PolyLsmEngine};
use rand::Rng;

fn main() {
    let num_vertices = 10_000;
    let num_edges = 10_000; // Smaller dataset to run both quickly in CI
    let mut rng = rand::thread_rng();
    let empty_props = HashMap::new();

    // Generate random pairs first to isolate storage time from random generation time
    let mut edge_pairs = Vec::with_capacity(num_edges as usize);
    for _ in 0..num_edges {
        let src = if rng.gen_bool(0.1) {
            rng.gen_range(1..=(num_vertices / 100).max(2))
        } else {
            rng.gen_range(1..=num_vertices)
        };
        let dst = rng.gen_range(1..=num_vertices);
        edge_pairs.push((src, dst));
    }

    // Run 1: Default PersistMode (OS Buffers)
    {
        let db_path = "test_db_benchmark_default";
        let _ = fs::remove_dir_all(db_path);
        let store = PolyLsmEngine::open(db_path).expect("Failed to open store");

        for i in 1..=num_vertices {
            store.put_vertex(i, &empty_props).unwrap();
        }

        let t_start = Instant::now();
        for &(src, dst) in &edge_pairs {
            store.put_edge(src, dst, &empty_props).unwrap();
        }
        let total_time = t_start.elapsed();
        let writes_per_sec = (num_edges as f64) / total_time.as_secs_f64();

        println!("\n[Phase 2 Gate] Default OS-Buffered Mode:");
        println!("  Total time  : {:?}", total_time);
        println!("  Throughput  : {:.2} writes/sec", writes_per_sec);
        
        let _ = fs::remove_dir_all(db_path);
    }

    // Run 2: Explicit PersistMode::SyncAll per write
    {
        let db_path = "test_db_benchmark_sync";
        let _ = fs::remove_dir_all(db_path);
        let store = PolyLsmEngine::open(db_path).expect("Failed to open store");

        for i in 1..=num_vertices {
            store.put_vertex(i, &empty_props).unwrap();
        }

        let t_start = Instant::now();
        for &(src, dst) in &edge_pairs {
            store.put_edge(src, dst, &empty_props).unwrap();
            store.flush().unwrap();
        }
        let total_time = t_start.elapsed();
        let writes_per_sec = (num_edges as f64) / total_time.as_secs_f64();

        println!("\n[Phase 2 Gate] Explicit PersistMode::SyncAll Mode (per write):");
        println!("  Total time  : {:?}", total_time);
        println!("  Throughput  : {:.2} writes/sec", writes_per_sec);
        
        let _ = fs::remove_dir_all(db_path);
    }
}
