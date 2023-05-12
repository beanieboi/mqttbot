use anyhow::Result;
use chrono::{DurationRound, Utc};
use tracing::info;

#[derive(Debug, serde::Deserialize)]
struct Vehicle {
    distance: f64,
}

pub async fn run(mqtt_client: &paho_mqtt::Client, client: &reqwest::Client) {
    let data = get_data(client).await.unwrap_or_else(|_| vec![]);
    let car_found = finder(data);

    let result = match car_found {
        Some(_) => "true".to_string(),
        None => "false".to_string(),
    };

    publish(
        mqtt_client,
        crate::mqtt::Payload {
            topic_suffix: "cityflitzer_nearby".to_string(),
            payload: result.clone(),
        },
    );

    publish(
        mqtt_client,
        crate::mqtt::Payload {
            topic_suffix: "update_date".to_string(),
            payload: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        },
    );

    info!(result);
}

fn finder(vehicles: Vec<Vehicle>) -> Option<()> {
    let max_distance = 500.0;
    let _ = vehicles.iter().find(|c| c.distance < max_distance)?;

    Some(())
}

async fn get_data(client: &reqwest::Client) -> Result<Vec<Vehicle>> {
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

    let resp = client
        .get(url)
        .header("X-API-KEY", "45d38969-0086-978d-dc06-7959b0d2fe79")
        .send()
        .await?
        .json::<Vec<Vehicle>>()
        .await?;
    Ok(resp)
}

fn publish(mqtt_client: &paho_mqtt::Client, payload: crate::mqtt::Payload) {
    mqtt_client
        .publish(payload.to_msg("mobility/cityflitzer"))
        .unwrap();
}
