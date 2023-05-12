#![allow(clippy::redundant_field_names)]
use std::time::Duration;
use tokio::{task, time};

mod cityflitzer;
mod mqtt;
mod nextbike;
mod raid;

#[tokio::main]
async fn main() {
    env_logger::init();
    let forever = task::spawn(async {
        let mut interval = time::interval(Duration::from_millis(100));
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_millis(1000))
            .build()
            .expect("failed to construct http client");

        loop {
            let mqtt_client = crate::mqtt::new_mqtt_client();
            let conn_opts = crate::mqtt::conn_opts();
            mqtt_client.connect(conn_opts).unwrap_or_else(|err| {
                panic!("Unable to connect: {:?}", err);
            });

            let _ = tokio::join!(
                nextbike::run(&mqtt_client, &http_client),
                cityflitzer::run(&mqtt_client, &http_client),
                raid::run(&mqtt_client),
                interval.tick()
            );
            mqtt_client
                .disconnect(paho_mqtt::DisconnectOptions::default())
                .expect("unable to disconnect")
        }
    });

    forever.await.unwrap()
}
