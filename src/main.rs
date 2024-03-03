use std::thread;
use actix_web::{web::Data, HttpServer, App};
use dotenv::dotenv;
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use listenfd::ListenFd;
mod errors;
mod userroutes;
mod midw;
mod kafka_consumer;
mod kafka_producer;


pub struct AppState {
    pub db: Pool<Postgres>,
}


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    let database_url = std::env::var("POSTGRES_URL_STAFF_USERS").expect("POSTGRES_URL_STAFF_USERS must be set");
    let pool = PgPoolOptions::new()
        .max_connections(1000)
        .connect(&database_url)
        .await
        .expect("Failed to create pool");
        let pool_clone = pool.clone();
        let kafka_consumer_thread = thread::spawn(move || {
            // Run the Kafka consumer within the Tokio runtime
            tokio::runtime::Runtime::new().unwrap().block_on(
                kafka_consumer::k_consumer(pool_clone),
            );
        });

        let mut listenfd = ListenFd::from_env();
        let mut server = HttpServer::new(move || App::new()
        .app_data(Data::new(AppState{db: pool.clone()}))
        .service(userroutes::fetchusers)
        .service(userroutes::fetchuserstwo)
);
        server = match listenfd.take_tcp_listener(0)? {
            Some(listener) => server.listen(listener)?,
            None => {
                let host = std::env::var("HOST_STAFF_USERS").expect("Please set host in .env");
                let port = std::env::var("PORT_STAFF_USERS").expect("Please set port in .env");
                server.bind(format!("{}:{}", host, port))?
            }
        };
        server.run().await.unwrap();

        kafka_consumer_thread.join().unwrap();
    
        Ok(())

        }
