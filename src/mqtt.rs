use std::env;

use paho_mqtt::ConnectOptions;

pub struct MqttClient {
    pub client: paho_mqtt::Client,
}

impl MqttClient {
    pub fn publish(&self, topic: String, payload: &str) -> Result<(), paho_mqtt::Error> {
        let msg = paho_mqtt::Message::new(topic, payload, 0);
        self.client.publish(msg)
    }
}

pub fn new_mqtt_client() -> MqttClient {
    let host = env::var("MQTT_HOST").unwrap_or_else(|_| "tcp://192.168.1.5:1883".to_string());
    let client = paho_mqtt::Client::new(host).unwrap_or_else(|err| {
        panic!("Error creating the client: {}", err);
    });

    MqttClient { client: client }
}

pub fn conn_opts() -> ConnectOptions {
    let username =
        env::var("MQTT_USERNAME").unwrap_or_else(|_| panic!("MQTT_USERNAME must be set"));
    let password =
        env::var("MQTT_PASSWORD").unwrap_or_else(|_| panic!("MQTT_PASSWORD must be set"));

    paho_mqtt::ConnectOptionsBuilder::new()
        .user_name(username)
        .password(password)
        .finalize()
}
