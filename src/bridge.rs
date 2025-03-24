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

use anyhow::{Context, Result};
use log::{error, info};
use std::path::PathBuf;
use tokio::{
    signal::{
        self,
        unix::{signal, SignalKind},
    },
    sync::mpsc::{self},
};

use crate::{
    dns_service::DnsService,
    listener::DnsListener,
    messaging::MessagePublisher,
    mqtt::{MqttClient, MqttMessage},
    mqtt_service::MqttService,
};

pub struct Bridge {
    dns_listener: Box<dyn DnsListener>,
    message_publisher: Option<Box<dyn MessagePublisher>>,
}

impl Bridge {
    pub fn new(dns_socket_path: PathBuf) -> Self {
        Self {
            dns_listener: Box::new(DnsService::new(dns_socket_path)),
            message_publisher: None,
        }
    }

    pub fn from_components(
        dns_listener: Box<dyn DnsListener>,
        message_publisher: Option<Box<dyn MessagePublisher>>,
    ) -> Self {
        Self {
            dns_listener,
            message_publisher,
        }
    }

    pub fn with_mqtt(
        dns_socket_path: PathBuf,
        mqtt_host: String,
        mqtt_port: u16,
        mqtt_username: String,
        mqtt_password: String,
        mqtt_topic_prefix: String,
    ) -> Self {
        let mqtt_client = MqttClient::new(
            mqtt_host.as_str(),
            mqtt_port,
            mqtt_username.as_str(),
            mqtt_password.as_str(),
        );
        info!("MQTT configured to {}:{}", mqtt_host, mqtt_port);

        let mqtt_service = MqttService::new(*mqtt_client.client.clone(), mqtt_topic_prefix);

        Self {
            dns_listener: Box::new(DnsService::new(dns_socket_path)),
            message_publisher: Some(Box::new(mqtt_service)),
        }
    }

    pub fn has_mqtt_config(&self) -> bool {
        self.message_publisher.is_some()
    }

    pub async fn start(&self) -> Result<()> {
        let (tx, mut rx) = mpsc::channel::<MqttMessage>(10);

        // Watch for interrupts so we can send death message via MQTT.
        let mut hup_signal = signal(SignalKind::hangup()).context("couldn't listen for SIGHUP")?;

        error!("Server ready");

        if let Some(ref publisher) = self.message_publisher {
            publisher.send_birth().await?;
        }

        // Start DNS listener in a separate task
        let dns_listener = self.dns_listener.box_clone();
        let tx_clone = tx.clone();
        let dns_task = tokio::spawn(async move { dns_listener.start_listening(tx_clone).await });

        loop {
            tokio::select! {
                _ = signal::ctrl_c() => {
                    break;
                },
                _ = hup_signal.recv() => {
                    break;
                },
                msg = rx.recv() => {
                    if let Some(message) = msg {
                        if let Some(ref publisher) = self.message_publisher {
                            if let Err(e) = publisher.publish(message).await {
                                error!("Failed to publish message: {}", e);
                            }
                        } else {
                            // Log message if no publisher is configured
                            match message {
                                MqttMessage::Publish { topic, payload } => {
                                    info!(
                                        "{}: {}",
                                        topic,
                                        String::from_utf8_lossy(payload.as_slice())
                                    );
                                }
                            }
                        }
                    }
                },
            }
        }

        if let Some(ref publisher) = self.message_publisher {
            publisher.send_death().await?;
        }

        // Cancel DNS listener task
        dns_task.abort();

        Ok(())
    }
}
