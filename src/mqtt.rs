use std::time::Duration;

use rumqttc::{AsyncClient, EventLoop, MqttOptions};

#[derive(Debug)]
pub enum MqttMessage {
    Publish { topic: String, payload: Vec<u8> },
}

pub struct MqttClient {
    pub client: Box<AsyncClient>,
    pub eventloop: Box<EventLoop>,
}

impl MqttClient {
    pub fn new(host: &str, port: u16, username: &str, password: &str) -> MqttClient {
        let client_id = "ring-detector".to_owned();

        let mut options = MqttOptions::new(client_id, host, port);
        options
            .set_credentials(username, password)
            .set_keep_alive(Duration::from_secs(30));

        let (client, eventloop) = AsyncClient::new(options, 10);

        Self {
            client: Box::new(client),
            eventloop: Box::new(eventloop),
        }
    }
}
