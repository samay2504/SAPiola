use crate::gen::sapiola::v1::CdcEvent;
use anyhow::{Context, Result};
use prost::Message;

pub fn deserialize_event(payload: &[u8]) -> Result<CdcEvent> {
    CdcEvent::decode(payload).context("Failed to decode CdcEvent")
}
