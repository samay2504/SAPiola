use std::collections::HashMap;
use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use poly_lsm_core::{GraphStore, PolyLsmEngine};
use rand::Rng;

fn main() {
    let db_path = "test_db_benchmark_adversarial";
    let _ = fs::remove_dir_all(db_path);

    // Initialize storage
    let (engine, rx) = PolyLsmEngine::open(db_path).expect("Failed to open store");
    let store = Arc::new(engine);
    let worker_store = Arc::clone(&store);
    tokio::runtime::Runtime::new().unwrap().spawn(async move {
        poly_lsm_core::engine::spawn_migration_worker(worker_store, rx).await;
    });

    let num_vertices = 100_000;
    let num_edges = 2_000_000;
    let num_writers = 4;
    let num_readers = 2;

    let empty_props = HashMap::new();

    println!("Initializing {} vertices...", num_vertices);
    for i in 1..=num_vertices {
        store.put_vertex("tenant1", i, &empty_props).unwrap();
    }

    println!(
        "Benchmarking ADVERSARIAL {} edge inserts across {} writers (90% writes to Node 1)...",
        num_edges, num_writers
    );

    let mut edge_pairs = Vec::with_capacity(num_edges as usize);
    let mut rng = rand::thread_rng();
    for _ in 0..num_edges {
        let src = if rng.gen_bool(0.9) {
            1 // 90% to Node 1
        } else {
            rng.gen_range(2..=num_vertices)
        };
        let dst = rng.gen_range(1..=num_vertices);
        edge_pairs.push((src, dst));
    }

    let chunk_size = (num_edges / num_writers) as usize;
    let mut writer_threads = Vec::new();

    let stop_readers = Arc::new(AtomicBool::new(false));
    let mut reader_threads = Vec::new();

    // Start background readers explicitly reading Node 1
    for _ in 0..num_readers {
        let store_clone = Arc::clone(&store);
        let stop_clone = Arc::clone(&stop_readers);
        
        let t = std::thread::spawn(move || {
            let mut latencies = Vec::new();
            
            while !stop_clone.load(Ordering::Relaxed) {
                let start = Instant::now();
                // 100% reads on Node 1 (the hub)
                let _ = store_clone.get_neighbors("tenant1", 1).unwrap();
                latencies.push(start.elapsed().as_micros());
                
                std::thread::yield_now();
            }
            latencies
        });
        reader_threads.push(t);
    }

    let start_time = Instant::now();

    for i in 0..num_writers {
        let store_clone = Arc::clone(&store);
        let chunk = edge_pairs[i * chunk_size..(i + 1) * chunk_size].to_vec();
        let empty_props_clone = empty_props.clone();
        
        let t = std::thread::spawn(move || {
            let mut latencies = Vec::with_capacity(chunk.len());
            for (src, dst) in chunk {
                let start = Instant::now();
                store_clone.put_edge("tenant1", src, dst, &empty_props_clone).unwrap();
                latencies.push(start.elapsed().as_micros());
            }
            latencies
        });
        writer_threads.push(t);
    }

    let mut all_write_latencies = Vec::with_capacity(num_edges as usize);
    for t in writer_threads {
        let lats = t.join().unwrap();
        all_write_latencies.extend(lats);
    }
    
    let total_time = start_time.elapsed();
    
    stop_readers.store(true, Ordering::Relaxed);
    let mut all_read_latencies = Vec::new();
    for t in reader_threads {
        let lats = t.join().unwrap();
        all_read_latencies.extend(lats);
    }

    all_write_latencies.sort_unstable();
    all_read_latencies.sort_unstable();

    let w_p50 = all_write_latencies[all_write_latencies.len() / 2];
    let w_p99 = all_write_latencies[(all_write_latencies.len() as f64 * 0.99) as usize];
    
    let r_p50 = all_read_latencies[all_read_latencies.len() / 2];
    let r_p99 = all_read_latencies[(all_read_latencies.len() as f64 * 0.99) as usize];

    println!("\n[Adversarial Benchmark] 90% writes to Node 1, 100% reads from Node 1:");
    println!("  Total edges       : {}", num_edges);
    println!("  Total time        : {:?}", total_time);
    println!("  Write Throughput  : {:.2} writes/sec", num_edges as f64 / total_time.as_secs_f64());
    println!("  Write p50 latency : {} µs", w_p50);
    println!("  Write p99 latency : {} µs", w_p99);
    println!("  Total reads done  : {}", all_read_latencies.len());
    println!("  Read p50 latency  : {} µs", r_p50);
    println!("  Read p99 latency  : {} µs", r_p99);
}
