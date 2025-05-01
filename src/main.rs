use std::time::Duration;
use tokio::{task, time};

mod cityflitzer;
mod hoymiles;
mod hoymiles_state;
mod mqtt;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let forever = task::spawn(async {
        let mut interval = time::interval(Duration::from_secs(120));
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_millis(5000))
            .build()
            .expect("failed to construct http client");
        let hm_state = hoymiles_state::init(&http_client).await;

        loop {
            let mqtt_client = crate::mqtt::new_mqtt_client();
            let conn_opts = crate::mqtt::conn_opts();
            mqtt_client.connect(conn_opts).unwrap_or_else(|err| {
                panic!("Unable to connect: {:?}", err);
            });

            let _ = tokio::join!(
                cityflitzer::run(&mqtt_client, &http_client),
                hoymiles::run(&mqtt_client, &http_client, &hm_state)
            );

            if mqtt_client.is_connected() {
                mqtt_client
                    .disconnect(paho_mqtt::DisconnectOptions::default())
                    .expect("unable to disconnect")
            }

            let _ = tokio::join!(interval.tick());
        }
    });

    forever.await.unwrap()
}
