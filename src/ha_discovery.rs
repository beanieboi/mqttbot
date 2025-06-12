use serde_json::json;
use tracing::error;

#[derive(Debug, Clone)]
pub struct Device {
    pub identifiers: Vec<String>,
    pub manufacturer: String,
    pub model: String,
    pub name: String,
    pub sw_version: String,
}

#[derive(Debug, Clone)]
pub struct SensorConfig {
    pub name: String,
    pub unique_id: String,
    pub state_topic: String,
    pub device: Device,
    pub unit_of_measurement: Option<String>,
    pub device_class: Option<String>,
    pub state_class: Option<String>,
    pub entity_category: Option<String>,
}

impl SensorConfig {
    /// Creates a new sensor configuration
    pub fn new(
        name: impl Into<String>,
        unique_id: impl Into<String>,
        state_topic: impl Into<String>,
        device: Device,
    ) -> Self {
        Self {
            name: name.into(),
            unique_id: unique_id.into(),
            state_topic: state_topic.into(),
            device,
            unit_of_measurement: None,
            device_class: None,
            state_class: None,
            entity_category: None,
        }
    }

    pub fn with_device_class(mut self, device_class: impl Into<String>) -> Self {
        self.device_class = Some(device_class.into());
        self
    }

    pub fn with_state_class(mut self, state_class: impl Into<String>) -> Self {
        self.state_class = Some(state_class.into());
        self
    }

    /// Converts the configuration to a JSON payload
    pub fn to_json(&self) -> serde_json::Value {
        let mut config = json!({
            "name": self.name,
            "unique_id": self.unique_id,
            "state_topic": self.state_topic,
            "device": {
                "identifiers": self.device.identifiers,
                "manufacturer": self.device.manufacturer,
                "model": self.device.model,
                "name": self.device.name,
                "sw_version": self.device.sw_version
            }
        });

        if let Some(unit) = &self.unit_of_measurement {
            config["unit_of_measurement"] = json!(unit);
        }
        if let Some(device_class) = &self.device_class {
            config["device_class"] = json!(device_class);
        }
        if let Some(state_class) = &self.state_class {
            config["state_class"] = json!(state_class);
        }
        if let Some(category) = &self.entity_category {
            config["entity_category"] = json!(category);
        }

        config
    }
}

pub fn publish_sensor_config(
    mqtt_client: &paho_mqtt::Client,
    discovery_prefix: &str,
    component_type: &str,
    config: &SensorConfig,
) {
    let discovery_topic = format!(
        "{}/{}/{}/config",
        discovery_prefix, component_type, config.unique_id
    );

    if let Err(e) = mqtt_client.publish(paho_mqtt::Message::new(
        discovery_topic,
        config.to_json().to_string(),
        0,
    )) {
        error!("Failed to publish discovery message: {}", e);
    }
}

/// Helper function to create a generic sensor configuration
pub fn create_sensor(
    name: impl Into<String>,
    unique_id: impl Into<String>,
    state_topic: impl Into<String>,
    device: Device,
) -> SensorConfig {
    SensorConfig::new(name, unique_id, state_topic, device)
}
