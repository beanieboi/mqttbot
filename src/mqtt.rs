use paho_mqtt::ConnectOptions;
use std::env;

pub struct Payload {
    pub topic_suffix: String,
    pub payload: String,
}

impl Payload {
    pub fn to_msg(&self, prefix: &str) -> paho_mqtt::Message {
        let topic = format!("{}/{}", prefix, self.topic_suffix);
        paho_mqtt::Message::new(topic, self.payload.to_owned(), 0)
    }
}

pub fn new_mqtt_client() -> paho_mqtt::Client {
    let host = env::var("MQTT_HOST").unwrap_or_else(|_| "tcp://192.168.1.5:1883".to_string());
    let co = paho_mqtt::CreateOptionsBuilder::new()
        .server_uri(host)
        .client_id("mqttbot")
        .finalize();

    paho_mqtt::Client::new(co).unwrap_or_else(|err| {
        panic!("Error creating the client: {}", err);
    })
}

pub fn conn_opts() -> ConnectOptions {
    let username =
        env::var("MQTT_USERNAME").unwrap_or_else(|_| panic!("MQTT_USERNAME must be set"));
    let password =
        env::var("MQTT_PASSWORD").unwrap_or_else(|_| panic!("MQTT_PASSWORD must be set"));

    paho_mqtt::ConnectOptionsBuilder::new()
        .user_name(username)
        .password(password)
        .connect_timeout(std::time::Duration::from_millis(100))
        .finalize()
}
