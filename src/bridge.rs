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
use log::{debug, error, info, trace, warn};
use rumqttc::{ConnectionError, QoS};
use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    net::UnixListener,
    signal::{
        self,
        unix::{signal, SignalKind},
    },
    sync::mpsc::{self, Sender},
    task,
    time::sleep,
};

use crate::{
    dns::DnsSocket,
    mqtt::{MqttClient, MqttMessage},
};

pub struct Bridge {
    dns_socket_path: PathBuf,
    mqtt_config: Option<MqttConfig>,
    doorbells: Arc<Mutex<HashSet<String>>>,
}

struct MqttConfig {
    mqtt_host: String,
    mqtt_port: u16,
    mqtt_username: String,
    mqtt_password: String,
    mqtt_topic_prefix: String,
}

impl Bridge {
    pub fn new(dns_socket_path: PathBuf) -> Self {
        Self {
            dns_socket_path,
            mqtt_config: None,
            doorbells: Arc::new(Mutex::new(HashSet::new())),
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
        Self {
            dns_socket_path,
            mqtt_config: Some(MqttConfig {
                mqtt_host,
                mqtt_port,
                mqtt_username,
                mqtt_password,
                mqtt_topic_prefix,
            }),
            doorbells: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub fn has_mqtt_config(&self) -> bool {
        self.mqtt_config.is_some()
    }

    pub async fn start(&self) -> Result<()> {
        let listener = UnixListener::bind(self.dns_socket_path.clone()).with_context(|| {
            format!(
                "Cannot bind to DNS listener {}",
                self.dns_socket_path.display()
            )
        })?;
        info!("listening on {}", self.dns_socket_path.display());

        let (tx, mut rx) = mpsc::channel(10);

        let mut mqtt_client = if let Some(c) = &self.mqtt_config {
            let mqtt_client = MqttClient::new(
                c.mqtt_host.as_str(),
                c.mqtt_port,
                c.mqtt_username.as_str(),
                c.mqtt_password.as_str(),
            );
            info!("MQTT configured to {}:{}", c.mqtt_host, c.mqtt_port);
            Some(mqtt_client)
        } else {
            None
        };

        // Watch for interrupts so we can send death message via MQTT.
        let mut hup_signal = signal(SignalKind::hangup()).context("couldn't listen for SIGHUP")?;

        error!("Server ready");

        if let Some(ref c) = mqtt_client {
            self.send_birth(*c.client.clone()).await;
        }

        loop {
            tokio::select! {
                _ = signal::ctrl_c() => {
                    break;
                },
                _ = hup_signal.recv() => {
                    break;
                },
                _ = self.accept_dns(&listener, tx.clone(), Arc::clone(&self.doorbells)) => {},
                v = self.check_mqtt_notifications(&mut mqtt_client) => {
                    if let Err(e) = v {
                        error!("Problem with MQTT: {}", e);
                        break;
                    }
                },
                v = rx.recv() => {
                    if let Some(ref mut c) = mqtt_client {
                        let client_copy = c.client.clone();
                        self.publish_message(*client_copy, v).await;
                    }
                },
            }
        }

        if let Some(c) = mqtt_client {
            self.send_death(*c.client.clone()).await;
        }

        Ok(())
    }

    async fn accept_dns(
        &self,
        listener: &UnixListener,
        sender: Sender<MqttMessage>,
        doorbells: Arc<Mutex<HashSet<String>>>,
    ) {
        let stream = match listener.accept().await {
            Ok((stream, _addr)) => stream,
            Err(e) => panic!("failure to connect: {}", e),
        };

        task::spawn(async move {
            let dns_socket = DnsSocket::new(stream.into_std().unwrap(), sender, doorbells);
            match dns_socket.handle_stream().await {
                Ok(_) => info!("server disconnected"),
                Err(err) => warn!("error on thread: {}", err),
            }
        });
    }

    async fn check_mqtt_notifications(
        &self,
        mqtt_client: &mut Option<MqttClient>,
    ) -> Result<(), ConnectionError> {
        if let Some(ref mut c) = mqtt_client {
            let event = match c.eventloop.poll().await {
                Ok(event) => event,
                Err(e) if matches!(e, ConnectionError::ConnectionRefused(_)) => {
                    return Err(e);
                }
                Err(e) => {
                    debug!("Problem connecting to MQTT: {:?}", e);
                    sleep(Duration::from_secs(1)).await;
                    return Ok(());
                }
            };
            debug!("mqtt: received {:?}", event);
        }
        Ok(())
    }

    async fn publish_message(&self, client: rumqttc::AsyncClient, msg: Option<MqttMessage>) {
        match msg {
            Some(MqttMessage::Publish {
                topic: topic_suffix,
                payload,
            }) => {
                if let Some(config) = &self.mqtt_config {
                    let topic = format!("{}/{}", config.mqtt_topic_prefix, topic_suffix);
                    trace!(
                        "Sending MQTT packet topic '{}' payload '{}'",
                        topic,
                        String::from_utf8_lossy(payload.as_slice()),
                    );
                    client
                        .publish(topic, QoS::AtLeastOnce, false, payload)
                        .await
                        .unwrap();
                } else {
                    info!(
                        "{}: {}",
                        topic_suffix,
                        String::from_utf8_lossy(payload.as_slice())
                    );
                }
            }
            None => todo!(),
        }
    }

    async fn send_birth(&self, client: rumqttc::AsyncClient) {
        info!("We have started");
        if let Some(c) = &self.mqtt_config {
            let _ = client
                .publish(
                    format!("{}/status", c.mqtt_topic_prefix),
                    QoS::AtLeastOnce,
                    true,
                    "online",
                )
                .await;
        }
    }

    async fn send_death(&self, client: rumqttc::AsyncClient) {
        info!("We are exiting");
        if let Some(c) = &self.mqtt_config {
            let _ = client
                .publish(
                    format!("{}/status", c.mqtt_topic_prefix),
                    QoS::AtLeastOnce,
                    true,
                    "offline",
                )
                .await;
            let _ = client.disconnect().await;
        }
    }
}
