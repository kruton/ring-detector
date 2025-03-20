/*
 * Copyright 2023 Kenny Root
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use rumqttc::{AsyncClient, EventLoop, MqttOptions};
use std::time::Duration;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mqtt_message_publish() {
        let topic = "test/topic".to_string();
        let payload = vec![1, 2, 3, 4];

        let message = MqttMessage::Publish {
            topic: topic.clone(),
            payload: payload.clone(),
        };

        let MqttMessage::Publish {
            topic: msg_topic,
            payload: msg_payload,
        } = message;
        assert_eq!(topic, msg_topic);
        assert_eq!(payload, msg_payload);
    }
}
