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

use super::{dnstap::Dnstap, mqtt::MqttMessage, net::parse_octets};
use anyhow::{anyhow, Context, Result};
use dns_parser::Packet as DnsPacket;
use fstrm::FstrmReader;
use log::{debug, info};
use prost::{bytes::BytesMut, Message};
use std::{
    collections::HashSet,
    io::Read,
    net::IpAddr,
    os::unix::net::UnixStream,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{sync::mpsc::Sender, time::Instant};

const HOST_EU: &str = "alarm.eu.s3.amazonaws.com";
const HOST_US: &str = "alarm.use.s3.amazonaws.com";

const IGNORE_DURATION: Duration = Duration::from_secs(1);

pub struct DnsSocket {
    stream: UnixStream,
    sender: Sender<MqttMessage>,
    start_time: Instant,
    doorbells: Arc<Mutex<HashSet<String>>>,
}

impl DnsSocket {
    pub fn new(
        stream: UnixStream,
        sender: Sender<MqttMessage>,
        doorbells: Arc<Mutex<HashSet<String>>>,
    ) -> Self {
        debug!("Ignoring packets from DNS server for {:?}", IGNORE_DURATION);
        Self {
            stream,
            sender,
            start_time: Instant::now() + IGNORE_DURATION,
            doorbells,
        }
    }

    pub async fn handle_stream(&self) -> Result<()> {
        info!("connected to DNS server");
        let _ = self.stream.set_nonblocking(false);
        let reader = FstrmReader::<_, ()>::new(&self.stream);
        let mut reader = reader.accept()?.start()?;
        debug!("FSTRM handshake finish {:?}", reader.content_types());

        while let Some(mut frame) = reader.read_frame()? {
            // DataFrame appears to have some kind of bug where UnixStream does not return
            // 0 so it hangs until the next byte?
            let mut buffer = BytesMut::new();
            buffer.resize(frame.size(), 0);
            frame
                .read_exact(&mut buffer)
                .with_context(|| format!("Expected to read {} bytes", frame.size()))?;

            if Instant::now() < self.start_time {
                debug!("Discarding frames until timer expires");
                continue;
            }

            match self.handle_frame(buffer).await {
                Err(e) => {
                    info!("Got invalid frame; Context: {}", e);
                }
                _ => (),
            }
        }
        Ok(())
    }

    async fn handle_frame(&self, buffer: BytesMut) -> Result<()> {
        let dnstap: Dnstap = Dnstap::decode(buffer)?;
        let msg = dnstap
            .message
            .context("dnstap frame did not have message")?;

        let client: IpAddr = parse_octets(msg.query_address()).context("invalid IP source")?;

        match msg.query_message {
            Some(query) => match DnsPacket::parse(&query.as_slice()) {
                Ok(packet) => self.handle_packet(&packet, client).await,
                Err(e) => Err(e.into()),
            },
            None => Err(anyhow!("Got empty query message")),
        }
    }

    fn get_config_message(&self, client: &String) -> MqttMessage {
        let topic = format!("ringdet-{}/config", client);
        let payload = "{}".as_bytes().to_vec();
        MqttMessage::Publish { topic, payload }
    }

    fn get_action_message(&self, client: &String) -> MqttMessage {
        let topic = format!("ringdet-{}/action", client);
        let payload = "{action:\"pressed\"}".as_bytes().to_vec();
        MqttMessage::Publish { topic, payload }
    }

    async fn handle_packet<'a>(&self, packet: &'a DnsPacket<'a>, client: IpAddr) -> Result<()> {
        let messages: Vec<MqttMessage> = packet
            .questions
            .iter()
            .filter(|q| {
                matches!(
                    q.qtype,
                    dns_parser::QueryType::A | dns_parser::QueryType::AAAA
                )
            })
            .filter_map(|q| {
                let name = &q.qname;
                match name.to_string().as_str() {
                    HOST_EU | HOST_US => {
                        debug!("we got {} from {:?}", name, &client);
                        let client_string = client.to_string();

                        let new_client = {
                            let mut lock = self.doorbells.lock().unwrap();
                            if lock.get(&client_string) == None {
                                lock.insert(client_string.clone());
                                true
                            } else {
                                false
                            }
                        };

                        let mut messages = vec![];
                        if new_client {
                            messages.push(self.get_config_message(&client_string));
                        }

                        messages.push(self.get_action_message(&client_string));

                        Some(messages)
                    }
                    _ => None,
                }
            })
            .flatten()
            .collect();

        for m in messages {
            self.sender.send(m).await.unwrap();
        }

        Ok(())
    }
}
