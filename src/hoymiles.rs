use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
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
    bms_soc: Option<String>,
    bms_in_eq: String,
    bms_out_eq: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct MeterData {
    today_eq: String,
    month_eq: String,
    year_eq: String,
    total_eq: String,
}

pub struct HoymilesState {
    token: Option<String>,
    sid: Option<String>,
}

impl HoymilesState {
    fn new() -> Self {
        Self {
            token: None,
            sid: None,
        }
    }
}
pub async fn init(client: &reqwest::Client) -> HoymilesState {
    let mut state = HoymilesState::new();
    if state.token.is_none() {
        info!("No token found. Requesting a new one.");
        state.token = request_new_token(client).await;
        if state.token.is_some() {
            info!("Successfully obtained new token");
        } else {
            error!("Failed to obtain token");
        }
    }
    if let Some(token) = &state.token {
        if state.sid.is_none() {
            info!("No sid found. Requesting a new one.");
            state.sid = get_sid(client, token).await;
            if state.sid.is_some() {
                info!("Successfully obtained SID");
            } else {
                error!("Failed to obtain SID");
            }
        }
    }

    state
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

    let sensors = [("soc", "Battery State of Charge", "battery", "%")];

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

async fn request_new_token(client: &reqwest::Client) -> Option<String> {
    let username = match std::env::var("HOYMILES_USERNAME") {
        Ok(u) => u,
        Err(e) => {
            error!("Failed to get HOYMILES_USERNAME: {}", e);
            return None;
        }
    };
    let password = match std::env::var("HOYMILES_PASSWORD") {
        Ok(p) => p,
        Err(e) => {
            error!("Failed to get HOYMILES_PASSWORD: {}", e);
            return None;
        }
    };

    // Get region
    let region_url = "https://euapi.hoymiles.com/iam/pub/0/c/region_c";
    let region_resp = match client
        .post(region_url)
        .json(&serde_json::json!({ "email": username }))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            error!("Failed to get region: {}", e);
            return None;
        }
    };

    let region_data: serde_json::Value = match region_resp.json().await {
        Ok(d) => d,
        Err(e) => {
            error!("Failed to parse region response: {}", e);
            return None;
        }
    };

    let login_url = match region_data["data"]["login_url"].as_str() {
        Some(url) => url,
        None => {
            error!("No login URL in region response");
            return None;
        }
    };

    // Generate password hash
    let md5_hash = format!("{:x}", md5::compute(password.as_bytes()));
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    let sha256_hash = BASE64.encode(hasher.finalize());
    let encoded_password = format!("{}.{}", md5_hash, sha256_hash);

    // Login
    let login_url = format!("{}/iam/pub/0/c/login_c", login_url);
    let login_resp = match client
        .post(login_url)
        .json(&serde_json::json!({
            "user_name": username,
            "password": encoded_password
        }))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            error!("Login request failed: {}", e);
            return None;
        }
    };

    let login_data: serde_json::Value = match login_resp.json().await {
        Ok(d) => d,
        Err(e) => {
            error!("Failed to parse login response: {}", e);
            return None;
        }
    };

    match login_data["data"]["token"].as_str() {
        Some(token) => Some(token.to_string()),
        None => {
            error!("No token in login response");
            None
        }
    }
}

async fn get_sid(client: &reqwest::Client, token: &str) -> Option<String> {
    let url = "https://neapi.hoymiles.com/pvmc/api/0/station/select_by_page_c";
    let resp = match client
        .post(url)
        .header("Authorization", token)
        .json(&serde_json::json!({
            "page": 1,
            "page_size": 50
        }))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            error!("Failed to get SID: {}", e);
            return None;
        }
    };

    let data: serde_json::Value = match resp.json().await {
        Ok(d) => d,
        Err(e) => {
            error!("Failed to parse SID response: {}", e);
            return None;
        }
    };

    match &data["data"]["list"][0]["sid"].as_number() {
        Some(sid) => Some(sid.to_string()),
        None => {
            error!("No SID in response");
            None
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

    let soc = data
        .data
        .unwrap()
        .reflux_station_data
        .unwrap()
        .bms_soc
        .unwrap();
    let payload = crate::mqtt::Payload {
        topic_suffix: "soc".to_string(),
        payload: soc,
    };
    publish(mqtt_client, payload);
}

fn publish(mqtt_client: &paho_mqtt::Client, payload: crate::mqtt::Payload) {
    if let Err(e) = mqtt_client.publish(payload.to_msg("solar/hoymiles")) {
        error!("Failed to publish MQTT message: {}", e);
    }
}
