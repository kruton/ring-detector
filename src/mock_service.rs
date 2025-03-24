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

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use mockall::mock;
    use mockall::predicate::*;
    use std::sync::{Arc, Mutex};

    // Mock the AsyncClient from rumqttc
    mock! {
        pub AsyncClient {}

        #[async_trait]
        impl rumqttc::AsyncClient for AsyncClient {
            async fn publish(&self, topic: String, qos: QoS, retain: bool, payload: Vec<u8>) -> Result<(), rumqttc::ClientError>;
            async fn disconnect(&self) -> Result<(), rumqttc::ClientError>;
        }
    }

    #[tokio::test]
    async fn test_publish() {
        // Setup
        let mut mock_client = MockAsyncClient::new();

        // Expect publish to be called with correct parameters
        mock_client
            .expect_publish()
            .with(
                eq("test_prefix/test_topic".to_string()),
                eq(QoS::AtLeastOnce),
                eq(false),
                eq(b"test_payload".to_vec()),
            )
            .returning(|_, _, _, _| Ok(()));

        let service = MqttService::new(mock_client, "test_prefix".to_string());

        let message = MqttMessage::Publish {
            topic: "test_topic".to_string(),
            payload: b"test_payload".to_vec(),
        };

        // Act
        let result = service.publish(message).await;

        // Assert
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_send_birth() {
        // Setup
        let mut mock_client = MockAsyncClient::new();

        // Expect publish to be called with correct parameters
        mock_client
            .expect_publish()
            .with(
                eq("test_prefix/status".to_string()),
                eq(QoS::AtLeastOnce),
                eq(true),
                eq(b"online".to_vec()),
            )
            .returning(|_, _, _, _| Ok(()));

        let service = MqttService::new(mock_client, "test_prefix".to_string());

        // Act
        let result = service.send_birth().await;

        // Assert
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_send_death() {
        // Setup
        let mut mock_client = MockAsyncClient::new();

        // Expect publish to be called with correct parameters
        mock_client
            .expect_publish()
            .with(
                eq("test_prefix/status".to_string()),
                eq(QoS::AtLeastOnce),
                eq(true),
                eq(b"offline".to_vec()),
            )
            .returning(|_, _, _, _| Ok(()));

        // Expect disconnect to be called
        mock_client.expect_disconnect().returning(|| Ok(()));

        let service = MqttService::new(mock_client, "test_prefix".to_string());

        // Act
        let result = service.send_death().await;

        // Assert
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_publish_error() {
        // Setup
        let mut mock_client = MockAsyncClient::new();

        // Mock a publish error
        mock_client.expect_publish().returning(|_, _, _, _| {
            Err(rumqttc::ClientError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Test error",
            )))
        });

        let service = MqttService::new(mock_client, "test_prefix".to_string());

        let message = MqttMessage::Publish {
            topic: "test_topic".to_string(),
            payload: b"test_payload".to_vec(),
        };

        // Act
        let result = service.publish(message).await;

        // Assert
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_clone() {
        // Setup
        let mock_client = MockAsyncClient::new();
        let service = MqttService::new(mock_client, "test_prefix".to_string());

        // Act
        let cloned_service = service.clone();

        // Assert - ensure the topic prefix is the same
        // Since we can't directly access the fields, we'll have to infer correctness
        assert_eq!(
            format!("{:?}", service.topic_prefix),
            format!("{:?}", cloned_service.topic_prefix)
        );
    }
}
