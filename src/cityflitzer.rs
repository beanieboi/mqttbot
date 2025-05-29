use anyhow::Result;
use chrono::{DurationRound, Utc};
use tracing::info;
use crate::ha_discovery::{create_sensor, publish_sensor_config, Device};

#[derive(Debug, serde::Deserialize)]
struct Vehicle {
    distance: f64,
}

pub async fn run(mqtt_client: &paho_mqtt::Client, client: &reqwest::Client) {
    publish_discovery(mqtt_client);
    
    let data = get_data(client).await.unwrap_or_else(|_| vec![]);
    let cars_found = finder(data);

    publish(
        mqtt_client,
        crate::mqtt::Payload {
            topic_suffix: "cityflitzer_nearby".to_string(),
            payload: cars_found.to_string(),
        },
    );

    publish(
        mqtt_client,
        crate::mqtt::Payload {
            topic_suffix: "update_date".to_string(),
            payload: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        },
    );

    info!(cars_found);
}

fn publish_discovery(mqtt_client: &paho_mqtt::Client) {
    let discovery_prefix = "homeassistant";
    let mqtt_topic = "mobility/cityflitzer";

    let device = Device {
        identifiers: vec!["cityflitzer-monitor".to_string()],
        manufacturer: "Cityflitzer".to_string(),
        model: "API Monitor".to_string(),
        name: "Cityflitzer Monitor".to_string(),
        sw_version: "1.0".to_string(),
    };

    let sensors = [
        create_sensor(
            "Nearby Cars",
            "cityflitzer_nearby",
            format!("{}/cityflitzer_nearby", mqtt_topic),
            device.clone(),
        )
        .with_state_class("measurement"),
        create_sensor(
            "Last Update",
            "cityflitzer_update",
            format!("{}/update_date", mqtt_topic),
            device.clone(),
        )
        .with_device_class("timestamp"),
    ];

    for sensor in sensors.iter() {
        publish_sensor_config(mqtt_client, discovery_prefix, "sensor", sensor);
    }
}

fn finder(vehicles: Vec<Vehicle>) -> usize {
    let max_distance = 500.0;
    vehicles
        .iter()
        .filter(|c| c.distance < max_distance)
        .count()
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
