use anyhow::Result;
use futures::StreamExt;
use rskafka::client::ClientBuilder;
use rskafka::record::OffsetAt;
use tracing::{info, warn};

pub struct CdcConsumer {
    brokers: String,
    group_id: String,
    topic: String,
}

impl CdcConsumer {
    pub fn new(brokers: &str, group_id: &str, topic: &str) -> Result<Self> {
        Ok(Self {
            brokers: brokers.to_string(),
            group_id: group_id.to_string(),
            topic: topic.to_string(),
        })
    }

    pub async fn run(&self) -> Result<()> {
        info!("Starting CDC consumer loop (brokers={}, topic={})", self.brokers, self.topic);
        
        let connection = self.brokers.clone();
        let client = ClientBuilder::new(vec![connection]).build().await?;
        let partition_client = client.partition_client(self.topic.clone(), 0).await?;

        let graph_server_url = std::env::var("GRAPH_SERVER_TARGET").unwrap_or_else(|_| "http://localhost:50053".to_string());
        let mut client = match crate::gen::sapiola::v1::storage_service_client::StorageServiceClient::connect(graph_server_url.clone()).await {
            Ok(c) => Some(c),
            Err(e) => {
                warn!("Failed to connect to graph server at {}: {}. Graph will not be populated.", graph_server_url, e);
                None
            }
        };

        let mut stream = partition_client.listen_to_offset(OffsetAt::Latest).await?;

        while let Some(Ok((record, _offset))) = stream.next().await {
            let payload = &record.value;
            
            use prost::Message as ProstMessage;
            match crate::gen::sapiola::v1::CdcEvent::decode(payload.as_slice()) {
                Ok(event) => {
                    info!("Decoded CdcEvent: table={}, pk={}", event.table, event.primary_key);
                    if let Some(ref mut c) = client {
                        let mut req = crate::gen::sapiola::v1::PutEdgeRequest {
                            source_id: 1,
                            target_id: 2,
                        };
                        
                        if let Ok(parsed_pk) = event.primary_key.parse::<u64>() {
                            req.source_id = parsed_pk;
                            req.target_id = parsed_pk + 1;
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

        Ok(())
    }
}
