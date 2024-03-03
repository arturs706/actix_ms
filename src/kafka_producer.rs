use std::time::Duration;
use rdkafka::{producer::{FutureProducer, FutureRecord, Producer}, ClientConfig};
use log::info;
use serde::{Deserialize, Serialize};



#[derive(Serialize, Deserialize, Clone, Debug)]
struct DataStruct {
    key: String,
    payload: String,
}

pub async fn produce(key_acc: &str, payload: String, topic_name: &str) -> Result<(), rdkafka::error::KafkaError> {
  let brokers = "localhost:9092";
    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", brokers)
        .set("message.timeout.ms", "5000")
        .create()
        .expect("Producer creation error");

    // Create an instance of your data struct
    let data = DataStruct {
        key: key_acc.to_string(),
        payload: payload.to_string()
    };

    // Serialize your data to a JSON string
    let json_data = serde_json::to_string(&data).expect("Failed to serialize data to JSON");

    let record = FutureRecord::to(&topic_name)
        .payload(&json_data)
        .key("key".as_bytes());

    let (partition, offset) = producer
        .send(record, Duration::from_secs(0))
        .await.expect("Message failed to be produced");

    info!(
        "Published message at topic '{}' partition '{}' offset '{}'",
        topic_name, partition, offset
    );
    producer.flush(Duration::from_secs(1)).expect("Flushing failed");
    Ok(())
}
