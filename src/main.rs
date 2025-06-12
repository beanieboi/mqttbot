use std::time::Duration;
use tokio::{task, time};
use tracing::{error, info, warn};

mod cityflitzer;
mod config;
mod ha_discovery;
mod hoymiles;
mod hoymiles_state;
mod mqtt;

async fn connect_with_retry(config: &config::Config) -> Option<paho_mqtt::Client> {
    let mut retry_count = 0;
    let mut backoff = config.general.initial_backoff().as_millis() as u64;

    loop {
        let mqtt_client = crate::mqtt::new_mqtt_client(&config.mqtt);
        let conn_opts = crate::mqtt::conn_opts(&config.mqtt);

        match mqtt_client.connect(conn_opts) {
            Ok(_) => {
                info!("Successfully connected to MQTT broker");
                return Some(mqtt_client);
            }
            Err(e) => {
                retry_count += 1;
                if retry_count >= config.general.max_retries {
                    error!(
                        "Failed to connect to MQTT broker after {} retries: {:?}",
                        config.general.max_retries, e
                    );
                    return None;
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
        let mut hm_state = hoymiles_state::init(&http_client, &config.hoymiles).await;

        loop {
            if let Some(mqtt_client) = connect_with_retry(&config).await {
                let _ = tokio::join!(
                    cityflitzer::run(&mqtt_client, &http_client, &config.cityflitzer),
                    hoymiles::run(&mqtt_client, &http_client, &mut hm_state, &config.hoymiles)
                );

                if mqtt_client.is_connected() {
                    if let Err(e) = mqtt_client.disconnect(paho_mqtt::DisconnectOptions::default())
                    {
                        error!("Error disconnecting from MQTT broker: {:?}", e);
                    }
                }
            } else {
                warn!("Skipping this iteration due to MQTT connection failure");
            }

            let _ = tokio::join!(interval.tick());
        }
    });

    forever.await.unwrap()
}
