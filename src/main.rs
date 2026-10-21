use std::time::Duration;
use tokio::{task, time};
use tracing::{error, info, warn};

mod cityflitzer;
mod config;
mod ha_discovery;
mod mqtt;

async fn connect_with_retry(mqtt_client: &paho_mqtt::Client, config: &config::Config) -> bool {
    let mut retry_count = 0;
    let mut backoff = config.general.initial_backoff().as_millis() as u64;

    loop {
        let conn_opts = crate::mqtt::conn_opts(&config.mqtt);

        match mqtt_client.connect(conn_opts) {
            Ok(_) => {
                info!("Successfully connected to MQTT broker");
                return true;
            }
            Err(e) => {
                retry_count += 1;
                if retry_count >= config.general.max_retries {
                    error!(
                        "Failed to connect to MQTT broker after {} retries: {:?}",
                        config.general.max_retries, e
                    );
                    return false;
                }
                warn!(
                    "Failed to connect to MQTT broker (attempt {}/{}): {:?}. Retrying in {}ms...",
                    retry_count, config.general.max_retries, e, backoff
                );
                tokio::time::sleep(Duration::from_millis(backoff)).await;
                backoff *= 2; // Exponential backoff
            }
        }
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config = match config::Config::from_env() {
        Ok(config) => config,
        Err(e) => {
            error!("Failed to load configuration: {:?}", e);
            std::process::exit(1);
        }
    };

    let forever = task::spawn(async move {
        let mut interval = time::interval(config.general.refresh_interval());
        let http_client = reqwest::Client::builder()
            .timeout(config.general.http_timeout())
            .build()
            .expect("failed to construct http client");

        // Keep a single client for the lifetime of the process. paho.mqtt.c
        // leaks a socketpair whenever its last client is destroyed, so creating
        // a new client per iteration eventually exhausts the fd limit.
        let mqtt_client = crate::mqtt::new_mqtt_client(&config.mqtt);

        loop {
            if connect_with_retry(&mqtt_client, &config).await {
                let _ = tokio::join!(cityflitzer::run(
                    &mqtt_client,
                    &http_client,
                    &config.cityflitzer
                ),);

                if mqtt_client.is_connected()
                    && let Err(e) = mqtt_client.disconnect(paho_mqtt::DisconnectOptions::default())
                {
                    error!("Error disconnecting from MQTT broker: {:?}", e);
                }
            } else {
                warn!("Skipping this iteration due to MQTT connection failure");
            }

            let _ = tokio::join!(interval.tick());
        }
    });

    forever.await.unwrap()
}
