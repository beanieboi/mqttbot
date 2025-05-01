use crate::hoymiles_state::HoymilesState;
use serde::{Deserialize, Serialize};
use tracing::{error, info};

#[derive(Debug, Deserialize, Serialize)]
struct HoymilesResponse {
    status: String,
    message: Option<String>,
    data: Option<HoymilesData>,
}

#[derive(Debug, Deserialize, Serialize)]
struct HoymilesData {
    data_time: String,
    last_data_time: String,
    is_reflux: i32,
    reflux_station_data: Option<RefluxStationData>,
}

#[derive(Debug, Deserialize, Serialize)]
struct RefluxStationData {
    start_date: String,
    end_date: String,
    pv_power: String,
    grid_power: String,
    load_power: String,
    bms_power: String,
    bms_soc: String,
    bms_in_eq: String,
    bms_out_eq: String,
}

pub async fn run(mqtt_client: &paho_mqtt::Client, client: &reqwest::Client, state: &HoymilesState) {
    publish_discovery(mqtt_client);

    if let Some(sid) = &state.sid {
        if let Some(token) = &state.token {
            get_station_data(client, mqtt_client, token, sid).await;
            info!(true);
        }
    }
}

fn publish_discovery(mqtt_client: &paho_mqtt::Client) {
    let discovery_prefix = "homeassistant";
    let mqtt_topic = "solar/hoymiles";

    let sensors = [
        ("bms_soc", "Battery State of Charge", "battery", "%"),
        ("bms_in_eq", "Battery Charge", "energy", "Wh"),
        ("bms_out_eq", "Battery Discharge", "energy", "Wh"),
    ];

    let device_info = serde_json::json!({
        "identifiers": ["hoymiles-ms-a2"],
        "manufacturer": "Hoymiles",
        "model": "MS-A2",
        "name": "Hoymiles MS-A2",
        "sw_version": "1.0"
    });

    for (sensor_id, name, device_class, unit) in sensors.iter() {
        let discovery_topic = format!(
            "{}/sensor/hoymiles-ms-a2/{}/config",
            discovery_prefix, sensor_id
        );
        let payload = serde_json::json!({
            "name": name,
            "unique_id": format!("hoymiles_{}", sensor_id),
            "state_topic": format!("{}/{}", mqtt_topic, sensor_id),
            "device": device_info,
            "unit_of_measurement": unit,
            "device_class": device_class,
            "state_class": if *device_class == "battery" { "MEASUREMENT" } else { "TOTAL_INCREASING" }
        });

        if let Err(e) = mqtt_client.publish(paho_mqtt::Message::new(
            discovery_topic,
            payload.to_string(),
            0,
        )) {
            error!("Failed to publish discovery message: {}", e);
        }
    }
}

async fn get_station_data(
    client: &reqwest::Client,
    mqtt_client: &paho_mqtt::Client,
    token: &str,
    sid: &str,
) {
    let url = "https://eud0.hoymiles.com/pvmc/api/0/station_data/real_g_c";
    let resp = match client
        .post(url)
        .header("Authorization", token)
        .json(&serde_json::json!({ "sid": sid }))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            error!("Failed to get station data: {}", e);
            return;
        }
    };

    let data = match resp.json::<HoymilesResponse>().await {
        Ok(d) => d,
        Err(e) => {
            error!("Failed to parse flow data response: {}", e);
            return;
        }
    };

    if let Some(hm_data) = data.data {
        if let Some(station_data) = hm_data.reflux_station_data {
            // Publish each sensor value
            let sensors = [
                ("bms_soc", &station_data.bms_soc),
                ("bms_in_eq", &station_data.bms_in_eq),
                ("bms_out_eq", &station_data.bms_out_eq),
            ];

            for (sensor_id, value) in sensors.iter() {
                let payload = crate::mqtt::Payload {
                    topic_suffix: sensor_id.to_string(),
                    payload: value.to_string(),
                };
                publish(mqtt_client, payload);
            }
        } else {
            error!("No reflux_station_data available");
        }
    } else {
        error!("No data in response");
    }
}

fn publish(mqtt_client: &paho_mqtt::Client, payload: crate::mqtt::Payload) {
    if let Err(e) = mqtt_client.publish(payload.to_msg("solar/hoymiles")) {
        error!("Failed to publish MQTT message: {}", e);
    }
}
