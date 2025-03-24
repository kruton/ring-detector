use crate::{messaging::MessagePublisher, mqtt::MqttMessage};
use async_trait::async_trait;
use std::sync::{Arc, Mutex};

#[derive(Default)]
pub struct MockPublisher {
    published_messages: Arc<Mutex<Vec<MqttMessage>>>,
    birth_called: Arc<Mutex<bool>>,
    death_called: Arc<Mutex<bool>>,
}

impl MockPublisher {
    pub fn new() -> Self {
        Self {
            published_messages: Arc::new(Mutex::new(Vec::new())),
            birth_called: Arc::new(Mutex::new(false)),
            death_called: Arc::new(Mutex::new(false)),
        }
    }

    pub fn get_published_messages(&self) -> Vec<MqttMessage> {
        self.published_messages.lock().unwrap().clone()
    }

    pub fn was_birth_called(&self) -> bool {
        *self.birth_called.lock().unwrap()
    }

    pub fn was_death_called(&self) -> bool {
        *self.death_called.lock().unwrap()
    }
}

#[async_trait]
impl MessagePublisher for MockPublisher {
    async fn publish(&self, message: MqttMessage) -> anyhow::Result<()> {
        self.published_messages.lock().unwrap().push(message);
        Ok(())
    }

    async fn send_birth(&self) -> anyhow::Result<()> {
        *self.birth_called.lock().unwrap() = true;
        Ok(())
    }

    async fn send_death(&self) -> anyhow::Result<()> {
        *self.death_called.lock().unwrap() = true;
        Ok(())
    }
}
