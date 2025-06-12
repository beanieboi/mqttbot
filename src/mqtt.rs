use crate::config::MqttConfig;
use paho_mqtt::ConnectOptions;

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

pub fn new_mqtt_client(config: &MqttConfig) -> paho_mqtt::Client {
    let co = paho_mqtt::CreateOptionsBuilder::new()
        .server_uri(&config.host)
        .client_id(&config.client_id)
        .finalize();

    paho_mqtt::Client::new(co).unwrap_or_else(|err| {
        panic!("Error creating the client: {}", err);
    })
}

pub fn conn_opts(config: &MqttConfig) -> ConnectOptions {
    paho_mqtt::ConnectOptionsBuilder::new()
        .user_name(&config.username)
        .password(&config.password)
        .connect_timeout(config.connect_timeout())
        .finalize()
}
