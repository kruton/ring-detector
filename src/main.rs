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

use ring_detector_lib::bridge::Bridge;

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

    let bridge = Bridge::new(
        cli.socket,
        cli.mqtt_host,
        cli.mqtt_port,
        cli.mqtt_username,
        cli.mqtt_password,
    );
    bridge.start().await
}
