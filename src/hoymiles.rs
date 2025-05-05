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
struct HoymilesErrorResponse {
    status: String,
    message: String,
    data: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize)]
struct HoymilesData {
    is_null: i32,
    today_eq: String,
    month_eq: String,
    year_eq: String,
    total_eq: String,
    real_power: String,
    co2_emission_reduction: String,
    plant_tree: String,
    data_time: String,
    last_data_time: String,
    capacitor: String,
    is_balance: i32,
    is_reflux: i32,
    reflux_station_data: Option<RefluxStationData>,
    clp: i32,
    efl_today_eq: Option<String>,
    efl_month_eq: Option<String>,
    efl_year_eq: Option<String>,
    efl_total_eq: Option<String>,
    electricity_price: f64,
    unit_code: String,
    unit: String,
    tou_mode: i32,
    is_load: i32,
    warn_data: WarnData,
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
    inv_num: i32,
    meter_location: i32,
    pv_to_load_eq: String,
    load_from_pv_eq: String,
    meter_b_in_eq: String,
    meter_b_out_eq: String,
    bms_in_eq: String,
    bms_out_eq: String,
    self_eq: String,
    pv_eq_total: String,
    use_eq_total: String,
    flows: Vec<String>,
    icon_pv: i32,
    icon_grid: i32,
    icon_load: i32,
    icon_bms: i32,
    icon_gen: i32,
    icon_pvi: i32,
    mb_in_eq: MeterEqData,
    mb_out_eq: MeterEqData,
    icon_plug: i32,
    icon_ai_plug: i32,
    cfg_load_power: i32,
}

#[derive(Debug, Deserialize, Serialize)]
struct MeterEqData {
    today_eq: String,
    month_eq: String,
    year_eq: String,
    total_eq: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct WarnData {
    s_uoff: bool,
}

#[derive(Debug)]
enum StationDataError {
    HttpRequestFailed(reqwest::Error),
    ReadBodyFailed(reqwest::Error),
    TokenExpired,
    ApiError(HoymilesErrorResponse),
    ParsingFailed {
        error: serde_json::Error,
        raw_body: String,
    },
    NoData,
}

pub async fn run(mqtt_client: &paho_mqtt::Client, client: &reqwest::Client, state: &mut HoymilesState) {
    publish_discovery(mqtt_client);

    if let Some(sid) = &state.sid {
        if let Some(token) = &state.token {
            match get_station_data(client, mqtt_client, token, sid).await {
                Ok(()) => {
                    info!("Successfully retrieved and published Hoymiles station data.");
                }
                Err(StationDataError::TokenExpired) => {
                    error!("Hoymiles token has expired. State refresh needed.");
                    state.refresh(client).await;
                }
                Err(StationDataError::HttpRequestFailed(e)) => {
                    error!("Failed to send request to Hoymiles API: {}", e);
                }
                Err(StationDataError::ReadBodyFailed(e)) => {
                    error!("Failed to read response body from Hoymiles API: {}", e);
                }
                Err(StationDataError::ApiError(api_err)) => {
                    error!("Hoymiles API returned an error: {:?}", api_err);
                }
                Err(StationDataError::ParsingFailed { error, raw_body }) => {
                    error!(
                        "Failed to parse Hoymiles response. Error: {}, Raw Body: {}",
                        error, raw_body
                    );
                }
                Err(StationDataError::NoData) => {
                    error!("No usable data found in Hoymiles response.");
                }
            }
        } else {
            error!("Hoymiles token is missing in state.");
        }
    } else {
        error!("Hoymiles SID is missing in state.");
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
) -> Result<(), StationDataError> {
    let url = "https://eud0.hoymiles.com/pvmc/api/0/station_data/real_g_c";
    let resp = match client
        .post(url)
        .header("Authorization", token)
        .json(&serde_json::json!({ "sid": sid }))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => return Err(StationDataError::HttpRequestFailed(e)),
    };

    let body_text = match resp.text().await {
        Ok(text) => text,
        Err(e) => return Err(StationDataError::ReadBodyFailed(e)),
    };

    match serde_json::from_str::<HoymilesResponse>(&body_text) {
        Ok(data) => {
            if let Some(hm_data) = data.data {
                if let Some(station_data) = hm_data.reflux_station_data {
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
                    Ok(())
                } else {
                    error!("No reflux_station_data available in response");
                    Err(StationDataError::NoData)
                }
            } else {
                error!("No 'data' field in response");
                Err(StationDataError::NoData)
            }
        }
        Err(initial_parse_err) => match serde_json::from_str::<HoymilesErrorResponse>(&body_text) {
            Ok(error_resp) => {
                if error_resp.status == "100" && error_resp.message == "token verify error." {
                    info!("Hoymiles token expired or invalid.");
                    Err(StationDataError::TokenExpired)
                } else {
                    error!(
                        "Hoymiles API returned an error: status={}, message='{}'",
                        error_resp.status, error_resp.message
                    );
                    Err(StationDataError::ApiError(error_resp))
                }
            }
            Err(_) => {
                error!(
                    "Failed to parse station data response as known structure: {}. Raw response: {}",
                    initial_parse_err, body_text
                );
                Err(StationDataError::ParsingFailed {
                    error: initial_parse_err,
                    raw_body: body_text,
                })
            }
        },
    }
}

fn publish(mqtt_client: &paho_mqtt::Client, payload: crate::mqtt::Payload) {
    if let Err(e) = mqtt_client.publish(payload.to_msg("solar/hoymiles")) {
        error!("Failed to publish MQTT message: {}", e);
    }
}
