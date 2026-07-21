use std::sync::Arc;
use fjall::{Config, KeyspaceCreateOptions};

fn main() {
    let db = Config::new("test_db_type").open().unwrap();
    let ks = db.keyspace("test", KeyspaceCreateOptions::default).unwrap();
    let r = ks.range(vec![1]..=vec![2]);
    let _: () = r;
}
