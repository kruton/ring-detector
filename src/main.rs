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
use clap::{Args, Parser};

use ring_detector_lib::bridge::Bridge;

#[derive(Parser)]
#[command(name = "ring-detector")]
/// Works with your DNS server to detect when EZVIZ doorbell button is activated.
struct Cli {
    #[arg(short = 's', long, env)]
    /// socket for dnstap listener
    dns_socket: std::path::PathBuf,

    #[command(flatten)]
    mqtt: MqttArgs,
}

#[derive(Args)]
#[group(requires_all = ["mqtt_host", "mqtt_port", "mqtt_username", "mqtt_password"], required = false)]
struct MqttArgs {
    #[arg(long, env)]
    /// MQTT hostname
    mqtt_host: Option<String>,

    #[arg(long, env)]
    /// MQTT port
    mqtt_port: Option<u16>,

    #[arg(long, env)]
    /// MQTT username
    mqtt_username: Option<String>,

    #[arg(long, env)]
    /// MQTT password
    mqtt_password: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::builder()
        .format_timestamp(Some(env_logger::TimestampPrecision::Millis))
        .init();

    let cli = Cli::parse();

    if cli.dns_socket.exists() {
        std::fs::remove_file(&cli.dns_socket)
            .with_context(|| format!("Cannot remove file {}", &cli.dns_socket.display()))?;
    }

    // Due to clap requires_all, if one MQTT parameter is there, they all are.
    let bridge = match cli.mqtt.mqtt_host {
        Some(host) => Bridge::with_mqtt(
            cli.dns_socket,
            host,
            cli.mqtt.mqtt_port.unwrap(),
            cli.mqtt.mqtt_username.unwrap(),
            cli.mqtt.mqtt_password.unwrap(),
        ),
        None => Bridge::new(cli.dns_socket),
    };

    bridge.start().await
}
