use std::env;
use std::time::Duration;
use tokio::{task, time};
use tracing::{error, info, warn};

mod cityflitzer;
mod ha_discovery;
mod hoymiles;
mod hoymiles_state;
mod mqtt;

const MAX_RETRIES: u32 = 10;
const INITIAL_BACKOFF_MS: u64 = 10000;

async fn connect_with_retry() -> Option<paho_mqtt::Client> {
    let mut retry_count = 0;
    let mut backoff = INITIAL_BACKOFF_MS;

    loop {
        let mqtt_client = crate::mqtt::new_mqtt_client();
        let conn_opts = crate::mqtt::conn_opts();

        match mqtt_client.connect(conn_opts) {
            Ok(_) => {
                info!("Successfully connected to MQTT broker");
                return Some(mqtt_client);
            }
            Err(e) => {
                retry_count += 1;
                if retry_count >= MAX_RETRIES {
                    error!(
                        "Failed to connect to MQTT broker after {} retries: {:?}",
                        MAX_RETRIES, e
                    );
                    return None;
                }
                warn!(
                    "Failed to connect to MQTT broker (attempt {}/{}): {:?}. Retrying in {}ms...",
                    retry_count, MAX_RETRIES, e, backoff
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

    let forever = task::spawn(async {
        let refresh_interval = env::var("REFRESH_INTERVAL").unwrap_or_else(|_| "120".to_string());
        let i = refresh_interval.parse::<u64>().unwrap_or(120);

        let mut interval = time::interval(Duration::from_secs(i));
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_millis(5000))
            .build()
            .expect("failed to construct http client");
        let mut hm_state = hoymiles_state::init(&http_client).await;

        loop {
            if let Some(mqtt_client) = connect_with_retry().await {
                let _ = tokio::join!(
                    cityflitzer::run(&mqtt_client, &http_client),
                    hoymiles::run(&mqtt_client, &http_client, &mut hm_state)
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
