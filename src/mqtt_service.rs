/*
 * Copyright 2025 Kenny Root
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
use crate::messaging::MessagePublisher;
use crate::mqtt::MqttMessage;
use async_trait::async_trait;
use rumqttc::{AsyncClient, QoS};

#[derive(Debug)]
pub struct MqttService {
    client: AsyncClient,
    topic_prefix: String,
}

impl Clone for MqttService {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            topic_prefix: self.topic_prefix.clone(),
        }
    }
}
impl MqttService {
    pub fn new(client: AsyncClient, topic_prefix: String) -> Self {
        Self {
            client,
            topic_prefix,
        }
    }
}

#[async_trait]
impl MessagePublisher for MqttService {
    async fn publish(&self, message: MqttMessage) -> anyhow::Result<()> {
        match message {
            MqttMessage::Publish {
                topic: topic_suffix,
                payload,
            } => {
                let topic = format!("{}/{}", self.topic_prefix, topic_suffix);
                self.client
                    .publish(topic, QoS::AtLeastOnce, false, payload)
                    .await?;
                Ok(())
            }
        }
    }

    async fn send_birth(&self) -> anyhow::Result<()> {
        self.client
            .publish(
                format!("{}/status", self.topic_prefix),
                QoS::AtLeastOnce,
                true,
                "online",
            )
            .await?;
        Ok(())
    }

    async fn send_death(&self) -> anyhow::Result<()> {
        self.client
            .publish(
                format!("{}/status", self.topic_prefix),
                QoS::AtLeastOnce,
                true,
                "offline",
            )
            .await?;
        self.client.disconnect().await?;
        Ok(())
    }
}
