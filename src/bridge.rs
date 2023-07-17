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
use rumqttc::{ConnectionError, EventLoop, QoS};
use std::{path::PathBuf, time::Duration};
use tokio::{
    net::UnixListener,
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
    mqtt_host: String,
    mqtt_port: u16,
    mqtt_username: String,
    mqtt_password: String,
}

impl Bridge {
    pub fn new(
        dns_socket_path: PathBuf,
        mqtt_host: String,
        mqtt_port: u16,
        mqtt_username: String,
        mqtt_password: String,
    ) -> Self {
        Self {
            dns_socket_path,
            mqtt_host,
            mqtt_port,
            mqtt_username,
            mqtt_password,
        }
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

        let mut mqtt_client = MqttClient::new(
            self.mqtt_host.as_str(),
            self.mqtt_port,
            self.mqtt_username.as_str(),
            self.mqtt_password.as_str(),
        );
        info!("MQTT configured to {}:{}", self.mqtt_host, self.mqtt_port);

        info!("Server ready");
        loop {
            tokio::select! {
                _ = self.accept_dns(&listener, tx.clone()) => {},
                v = self.check_mqtt_notifications(&mut mqtt_client.eventloop) => {
                    if let Err(e) = v {
                        error!("Problem with MQTT: {}", e);
                        break;
                    }
                },
                v = rx.recv() => self.publish_message(&mqtt_client, v).await,
            }
        }

        Ok(())
    }

    async fn accept_dns(&self, listener: &UnixListener, sender: Sender<MqttMessage>) {
        let stream = match listener.accept().await {
            Ok((stream, _addr)) => stream,
            Err(e) => panic!("failure to connect: {}", e),
        };

        task::spawn(async move {
            let dns_socket = DnsSocket::new(stream.into_std().unwrap(), sender);
            match dns_socket.handle_stream().await {
                Ok(_) => info!("unbound disconnected"),
                Err(err) => warn!("error on thread: {}", err),
            }
        });
    }

    async fn check_mqtt_notifications(
        &self,
        eventloop: &mut EventLoop,
    ) -> Result<(), ConnectionError> {
        let event = match eventloop.poll().await {
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
        Ok(())
    }

    async fn publish_message(&self, client: &MqttClient, msg: Option<MqttMessage>) {
        match msg {
            Some(MqttMessage::Publish { topic, payload }) => {
                trace!(
                    "Sending MQTT packet topic '{}' payload '{}'",
                    topic,
                    String::from_utf8_lossy(payload.as_slice()),
                );
                client
                    .client
                    .publish(topic, QoS::AtLeastOnce, false, payload)
                    .await
                    .unwrap();
            }
            None => todo!(),
        }
    }
}
