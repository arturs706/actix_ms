use rdkafka::client::ClientContext;
use rdkafka::config::{ClientConfig, RDKafkaLogLevel};
use rdkafka::consumer::stream_consumer::StreamConsumer;
use rdkafka::consumer::{CommitMode, Consumer, ConsumerContext, Rebalance};
use rdkafka::error::KafkaResult;
use rdkafka::message::Message;
use rdkafka::topic_partition_list::TopicPartitionList;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, Pool, Postgres};
use crate::kafka_producer;

#[derive(Deserialize, Serialize, FromRow, Debug)]
struct User {
    name: String,
}

struct CustomContext;

impl ClientContext for CustomContext {
    const ENABLE_REFRESH_OAUTH_TOKEN: bool = false;
}

impl ConsumerContext for CustomContext {
    fn pre_rebalance(&self, rebalance: &Rebalance) {
        info!("Pre rebalance {:?}", rebalance);
    }

    fn post_rebalance(&self, rebalance: &Rebalance) {
        info!("Post rebalance {:?}", rebalance);
    }

    fn commit_callback(&self, result: KafkaResult<()>, _offsets: &TopicPartitionList) {
        info!("Committing offsets: {:?}", result);
    }
}

#[derive(Deserialize)]
struct DataStruct {
    key: String,
    payload: String,
}

type LoggingConsumer = StreamConsumer<CustomContext>;

pub async fn k_consumer(db: Pool<Postgres>) {
    let brokers = "localhost:9092";
    let group_id: &str = "consumer-group-a";
    let topic = "post_get_user";
    let context = CustomContext;

    let consumer: LoggingConsumer = ClientConfig::new()
        .set("group.id", group_id)
        .set("bootstrap.servers", brokers)
        .set("enable.partition.eof", "false")
        .set("session.timeout.ms", "6000")
        .set("enable.auto.commit", "true")
        .set("auto.offset.reset", "earliest")
        .set_log_level(RDKafkaLogLevel::Debug)
        .create_with_context(context)
        .expect("Consumer creation failed");

    consumer
        .subscribe(&[topic])
        .expect("Can't subscribe to specified topic");

    loop {
        match consumer.recv().await {
            Err(e) => {
                warn!("Kafka error: {}", e);
                // Consumer might be disconnected if an error occurs
                consumer.commit_consumer_state(CommitMode::Async).unwrap();
            }
            Ok(m) => {
                let payload = match m.payload_view::<str>() {
                    None => "",
                    Some(Ok(s)) => s,
                    Some(Err(e)) => {
                        warn!("Error while deserializing message payload: {:?}", e);
                        ""
                    }
                };
                let user_serialized = serde_json::from_str::<DataStruct>(&payload).unwrap();
                let mut value: Value = serde_json::from_str(&user_serialized.payload).unwrap();

                if let Some(created_by) = value.get("user_reg").and_then(|user_reg| user_reg.get("createdby")) {
                    if let Some(created_by_str) = created_by.as_str() {
                        if user_serialized.key == "post_get_user" && m.topic() == "post_get_user" {
                            let fetch_user = sqlx::query_as::<_, User>("SELECT name FROM staff_users WHERE user_id = $1")
                                .bind(created_by_str)
                                .fetch_one(&db)
                                .await;

                            match fetch_user {
                                Ok(retrieved_user) => {
                                    value["user_reg"]["createdby"] = serde_json::Value::String(retrieved_user.name);
                                    println!("Value: {}", serde_json::to_string(&value).unwrap());
                                    kafka_producer::produce("response_user", serde_json::to_string(&value).unwrap(), "register_landlords").await.expect("Failed to produce message to Kafka")
                                }
                                Err(err) => {
                                    warn!("Error deserializing JSON payload: {:?}", err);
                                }
                            }
                        }
                    } else {
                        println!("createdby is not a string.");
                    }
                } else {
                    println!("user_reg or createdby not found in the JSON structure.");
                }
                consumer.commit_message(&m, CommitMode::Async).unwrap();
            }
        };
    }
}
