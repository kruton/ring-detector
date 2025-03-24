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

use anyhow::Context;
use async_trait::async_trait;
use log::{info, warn};
use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tokio::{net::UnixListener, sync::mpsc::Sender, task};

use crate::{dns::DnsSocket, listener::DnsListener, mqtt::MqttMessage};

#[derive(Debug, Clone)]
pub struct DnsService {
    socket_path: PathBuf,
    doorbells: Arc<Mutex<HashSet<String>>>,
}

impl DnsService {
    pub fn new(socket_path: PathBuf) -> Self {
        Self {
            socket_path,
            doorbells: Arc::new(Mutex::new(HashSet::new())),
        }
    }
}

#[async_trait]
impl DnsListener for DnsService {
    async fn start_listening(&self, message_sender: Sender<MqttMessage>) -> anyhow::Result<()> {
        let listener = UnixListener::bind(&self.socket_path).with_context(|| {
            format!("Cannot bind to DNS listener {}", self.socket_path.display())
        })?;
        info!("listening on {}", self.socket_path.display());

        loop {
            let (stream, _) = listener.accept().await?;
            let sender = message_sender.clone();
            let doorbells = Arc::clone(&self.doorbells);

            task::spawn(async move {
                let dns_socket = DnsSocket::new(stream.into_std().unwrap(), sender, doorbells);
                match dns_socket.handle_stream().await {
                    Ok(_) => info!("server disconnected"),
                    Err(err) => warn!("error on thread: {}", err),
                }
            });
        }
    }

    fn box_clone(&self) -> Box<dyn DnsListener + Send + Sync + 'static> {
        Box::new(Self {
            socket_path: self.socket_path.clone(),
            doorbells: Arc::clone(&self.doorbells),
        })
    }
}
