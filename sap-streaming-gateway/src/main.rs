use anyhow::Result;
use sap_streaming_gateway::consumer::CdcConsumer;
use tracing::info;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    info!("Starting sap-streaming-gateway");

    // Redpanda or Kafka broker
    let brokers = std::env::var("KAFKA_BROKERS").unwrap_or_else(|_| "localhost:9092".into());
    let topic = std::env::var("KAFKA_TOPIC").unwrap_or_else(|_| "sap.cdc.events".into());
    let group_id = std::env::var("KAFKA_GROUP_ID").unwrap_or_else(|_| "sap-streaming-gateway-group".into());

    let consumer = CdcConsumer::new(&brokers, &group_id, &topic)?;
    
    // In a real implementation, we would spawn this and also start a gRPC server or internal orchestrator
    consumer.run().await?;

    Ok(())
}
