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
use clap::Parser;
use log::{debug, error, info, trace, warn};
use ring_detector::{
    dns::DnsSocket,
    mqtt::{MqttClient, MqttMessage},
};
use rumqttc::{ConnectionError, EventLoop, QoS};
use tokio::{
    net::UnixListener,
    sync::mpsc,
    sync::mpsc::Sender,
    task,
    time::{sleep, Duration},
};

#[derive(Parser)]
#[command(name = "ring-detector")]
/// Works with your DNS server to detect when EZVIZ doorbell button is activated.
struct Cli {
    #[arg(short, long)]
    /// socket for dnstap listener
    socket: std::path::PathBuf,

    #[arg(long)]
    /// MQTT hostname
    mqtt_host: String,

    #[arg(long)]
    /// MQTT port
    mqtt_port: u16,

    #[arg(long)]
    /// MQTT username
    mqtt_username: String,

    #[arg(long)]
    /// MQTT password
    mqtt_password: String,
}

async fn accept_dns(listener: &UnixListener, sender: Sender<MqttMessage>) {
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

async fn check_mqtt_notifications(eventloop: &mut EventLoop) -> Result<(), ConnectionError> {
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

async fn publish_message(client: &MqttClient, msg: Option<MqttMessage>) {
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

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::builder()
        .format_timestamp(Some(env_logger::TimestampPrecision::Millis))
        .init();

    let cli = Cli::parse();

    if cli.socket.exists() {
        std::fs::remove_file(&cli.socket)
            .with_context(|| format!("Cannot remove file {}", &cli.socket.display()))?;
    }

    let listener = UnixListener::bind(&cli.socket)
        .with_context(|| format!("Cannot bind to DNS listener {}", &cli.socket.display()))?;
    info!("listening on {}", cli.socket.display());

    let (tx, mut rx) = mpsc::channel(10);

    let mut mqtt_client = MqttClient::new(
        cli.mqtt_host.as_str(),
        cli.mqtt_port,
        cli.mqtt_username.as_str(),
        cli.mqtt_password.as_str(),
    );
    info!("MQTT configured to {}:{}", cli.mqtt_host, cli.mqtt_port);

    loop {
        tokio::select! {
            _ = accept_dns(&listener, tx.clone()) => {},
            v = check_mqtt_notifications(&mut mqtt_client.eventloop) => {
                if let Err(e) = v {
                    error!("Problem with MQTT: {}", e);
                    break;
                }
            },
            v = rx.recv() => publish_message(&mqtt_client, v).await,
        }
    }

    Ok(())
}
