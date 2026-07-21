use crate::gen::sapiola::v1::CdcEvent;
use anyhow::Result;
use tracing::info;

pub struct Dispatcher;

impl Dispatcher {
    pub fn dispatch(event: &CdcEvent) -> Result<()> {
        info!("Dispatching event for table: {}, pk: {}", event.table, event.primary_key);
        // Route to Poly-LSM / Graph Engine
        Ok(())
    }
}
