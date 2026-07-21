use anyhow::Result;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::{ClientConfig, Message};
use tracing::{info, warn};

pub struct CdcConsumer {
    consumer: StreamConsumer,
}

impl CdcConsumer {
    pub fn new(brokers: &str, group_id: &str, topic: &str) -> Result<Self> {
        let consumer: StreamConsumer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("group.id", group_id)
            .set("enable.partition.eof", "false")
            .set("session.timeout.ms", "6000")
            .set("enable.auto.commit", "true")
            .create()?;

        consumer.subscribe(&[topic])?;

        Ok(Self { consumer })
    }

    pub async fn run(&self) -> Result<()> {
        info!("Starting CDC consumer loop");
        
        let graph_server_url = std::env::var("GRAPH_SERVER_TARGET").unwrap_or_else(|_| "http://localhost:50052".to_string());
        let mut client = match crate::gen::sapiola::v1::storage_service_client::StorageServiceClient::connect(graph_server_url.clone()).await {
            Ok(c) => Some(c),
            Err(e) => {
                warn!("Failed to connect to graph server at {}: {}. Graph will not be populated.", graph_server_url, e);
                None
            }
        };

        loop {
            match self.consumer.recv().await {
                Err(e) => warn!("Kafka error: {}", e),
                Ok(m) => {
                    let payload = match m.payload_view::<[u8]>() {
                        None => continue,
                        Some(Err(e)) => {
                            warn!("Error while deserializing message payload: {:?}", e);
                            continue;
                        }
                        Some(Ok(bytes)) => bytes,
                    };
                    
                    use prost::Message as ProstMessage;
                    match crate::gen::sapiola::v1::CdcEvent::decode(payload) {
                        Ok(event) => {
                            info!("Decoded CdcEvent: table={}, pk={}", event.table, event.primary_key);
                            if let Some(ref mut c) = client {
                                // V1 Hack: Create edges blindly from CDC events to demonstrate E2E data flow.
                                // In production, we'd use the DSL mapper to correctly assign src/dst.
                                let mut req = crate::gen::sapiola::v1::PutEdgeRequest {
                                    source_id: 1,
                                    target_id: 2,
                                };
                                
                                // Actually, we should parse the PK to get unique IDs if possible
                                if let Ok(parsed_pk) = event.primary_key.parse::<u64>() {
                                    req.source_id = parsed_pk;
                                    req.target_id = parsed_pk + 1; // Fake edge for RAG path
                                } else if event.primary_key.contains('|') {
                                    let parts: Vec<&str> = event.primary_key.split('|').collect();
                                    if parts.len() == 2 {
                                        if let (Ok(s), Ok(t)) = (parts[0].parse::<u64>(), parts[1].parse::<u64>()) {
                                            req.source_id = s;
                                            req.target_id = t;
                                        }
                                    }
                                }

                                let mut request = tonic::Request::new(req);
                                request.metadata_mut().insert("tenant-id", "default".parse().unwrap());
                                if let Err(e) = c.put_edge(request).await {
                                    warn!("Failed to write to graph server: {}", e);
                                }
                            }
                        }
                        Err(e) => warn!("Failed to decode protobuf CdcEvent: {:?}", e),
                    }
                }
            };
        }
    }
}
