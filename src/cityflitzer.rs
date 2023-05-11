use anyhow::Result;
use chrono::{DurationRound, Utc};
use std::time::Duration;

#[derive(Debug, serde::Deserialize)]
struct Vehicle {
    distance: f64,
}

pub async fn run() {
    let mqtt_client = crate::mqtt::new_mqtt_client();
    let conn_opts = crate::mqtt::conn_opts();
    mqtt_client.client.connect(conn_opts).unwrap_or_else(|err| {
        panic!("Unable to connect: {:?}", err);
    });

    let data = get_data().await.unwrap_or_else(|_| vec![]);
    let car_found = finder(data);

    publish(
        &mqtt_client,
        "cityflitzer_nearby",
        match car_found {
            true => "true",
            false => "false",
        },
    );
    publish(
        &mqtt_client,
        "update_date",
        &Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
    );
}

fn finder(data: Vec<Vehicle>) -> bool {
    let max_distance = 500.0;
    let mut found_nearby = false;
    for vehicle in data {
        if vehicle.distance < max_distance {
            found_nearby = true;
            break;
        }
    }
    found_nearby
}

async fn get_data() -> Result<Vec<Vehicle>> {
    let client = reqwest::Client::builder()
        .timeout(Duration::new(1, 0))
        .build();

    let lat = 51.32032033409821;
    let long = 12.36535400104385;
    let now = chrono::Utc::now();
    let start = now.duration_trunc(chrono::Duration::hours(1)).unwrap();
    let end = start + chrono::Duration::hours(1);

    let url = format!(
      "https://de1.cantamen.de/casirest/v3/pointsofinterest?placeIsFixed=false&lat={lat}&lng={lng}&range=30000&start={start}&end={end}&sort=distance",
      lat = lat,
      lng = long,
      start = start.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
      end = end.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
    );

    let resp = client?
        .get(url)
        .header("X-API-KEY", "45d38969-0086-978d-dc06-7959b0d2fe79")
        .send()
        .await?
        .json::<Vec<Vehicle>>()
        .await?;
    Ok(resp)
}

fn publish(mqtt_client: &crate::mqtt::MqttClient, topic_suffix: &str, payload: &str) {
    let topic_prefix = "mobility/cityflitzer1";
    let topic = format!("{}/{}", topic_prefix, topic_suffix);
    mqtt_client.publish(topic, payload).unwrap();
}
