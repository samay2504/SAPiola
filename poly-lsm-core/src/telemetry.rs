use fjall::compaction::filter::{
    CompactionFilter, Context, Factory, ItemAccessor, Verdict,
};
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[derive(Clone)]
pub struct Telemetry {
    pub compactions_topology: Arc<AtomicUsize>,
    pub compactions_vprop: Arc<AtomicUsize>,
    pub compactions_eprop: Arc<AtomicUsize>,
}

impl Telemetry {
    pub fn new() -> Self {
        Self {
            compactions_topology: Arc::new(AtomicUsize::new(0)),
            compactions_vprop: Arc::new(AtomicUsize::new(0)),
            compactions_eprop: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn get_disk_size<P: AsRef<Path>>(path: P) -> u64 {
        let mut total_size = 0;
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    if metadata.is_dir() {
                        total_size += Self::get_disk_size(entry.path());
                    } else {
                        total_size += metadata.len();
                    }
                }
            }
        }
        total_size
    }
}

pub struct TelemetryFilterFactory {
    counter: Arc<AtomicUsize>,
}

impl TelemetryFilterFactory {
    pub fn new(counter: Arc<AtomicUsize>) -> Self {
        Self { counter }
    }
}

impl Factory for TelemetryFilterFactory {
    fn name(&self) -> &str {
        "telemetry_filter"
    }

    fn make_filter(&self, _ctx: &Context) -> Box<dyn CompactionFilter + 'static> {
        Box::new(TelemetryFilter {
            counter: self.counter.clone(),
        })
    }
}

pub struct TelemetryFilter {
    counter: Arc<AtomicUsize>,
}

impl CompactionFilter for TelemetryFilter {
    fn filter_item(
        &mut self,
        _item: ItemAccessor<'_>,
        _ctx: &Context,
    ) -> Result<Verdict, lsm_tree::Error> {
        self.counter.fetch_add(1, Ordering::Relaxed);
        Ok(Verdict::Keep)
    }
}
