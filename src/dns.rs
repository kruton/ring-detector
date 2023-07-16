use super::{dnstap::Dnstap, mqtt::MqttMessage, net::parse_octets};
use anyhow::{anyhow, Context, Result};
use dns_parser::Packet as DnsPacket;
use fstrm::FstrmReader;
use log::{debug, info};
use prost::{bytes::BytesMut, Message};
use std::{io::Read, net::IpAddr, os::unix::net::UnixStream, time::Duration};
use tokio::{sync::mpsc::Sender, time::Instant};

const HOST_EU: &str = "alarm.eu.s3.amazonaws.com";
const HOST_US: &str = "alarm.use.s3.amazonaws.com";

const IGNORE_DURATION: Duration = Duration::from_secs(1);

pub struct DnsSocket {
    stream: UnixStream,
    sender: Sender<MqttMessage>,
    start_time: Instant,
}

impl DnsSocket {
    pub fn new(stream: UnixStream, sender: Sender<MqttMessage>) -> Self {
        debug!("Ignoring packets from DNS server for {:?}", IGNORE_DURATION);
        Self {
            stream,
            sender,
            start_time: Instant::now() + IGNORE_DURATION,
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
                        let topic = format!("homeassistant/ring-{}/action", client);
                        let payload = "{action:\"pressed\"}".as_bytes().to_vec();

                        Some(MqttMessage::Publish { topic, payload })
                    }
                    _ => None,
                }
            })
            .collect();

        for m in messages {
            self.sender.send(m).await.unwrap();
        }

        Ok(())
    }
}