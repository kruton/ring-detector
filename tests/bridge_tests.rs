use ring_detector_lib::bridge::Bridge;
use tempfile::tempdir;

#[test]
fn test_bridge_creation() {
    let dir = tempdir().unwrap();
    let socket_path = dir.path().join("dns.sock");

    // Test without MQTT
    let bridge = Bridge::new(socket_path.clone());
    assert!(!bridge.has_mqtt_config());

    // Test with MQTT
    let bridge_with_mqtt = Bridge::with_mqtt(
        socket_path,
        "localhost".to_string(),
        1883,
        "user".to_string(),
        "pass".to_string(),
        "prefix".to_string(),
    );

    assert!(bridge_with_mqtt.has_mqtt_config());
}

use anyhow::Result;
use async_trait::async_trait;
use mockall::predicate::*;
use mockall::*;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::{self, Sender};

use ring_detector_lib::listener::DnsListener;
use ring_detector_lib::messaging::MessagePublisher;
use ring_detector_lib::mqtt::MqttMessage;

// Create mock for DnsListener
mock! {
    #[derive(Debug)]
    pub DnsListener {}

    #[async_trait]
    #[derive(Debug)]
    impl DnsListener for DnsListener {
        async fn start_listening(&self, message_sender: Sender<MqttMessage>) -> Result<()>;
        fn box_clone(&self) -> Box<dyn DnsListener + Send + Sync>;
    }

    impl Clone for DnsListener {
        fn clone(&self) -> Self;
    }
}

// Create mock for MessagePublisher
mock! {
    #[derive(Debug)]
    pub MessagePublisher {}

    #[async_trait]
    impl MessagePublisher for MessagePublisher {
        async fn publish(&self, message: MqttMessage) -> Result<()>;
        async fn send_birth(&self) -> Result<()>;
        async fn send_death(&self) -> Result<()>;
    }
}

#[tokio::test]
async fn async_test_bridge_creation() {
    let temp_dir = tempfile::tempdir().unwrap();
    let socket_path = temp_dir.path().join("dns.sock");

    // Test without MQTT
    let bridge = Bridge::new(socket_path.clone());
    assert!(!bridge.has_mqtt_config());

    // Test with MQTT
    let bridge_with_mqtt = Bridge::with_mqtt(
        socket_path,
        "localhost".to_string(),
        1883,
        "user".to_string(),
        "pass".to_string(),
        "prefix".to_string(),
    );

    assert!(bridge_with_mqtt.has_mqtt_config());
}

#[tokio::test]
async fn test_bridge_start_with_mqtt() {
    // Create mocks
    let mut mock_listener = MockDnsListener::new();
    let mut mock_publisher = MockMessagePublisher::new();

    // Configure the mocks for expected behavior
    // Message channel to simulate DNS messages
    let (_tx, _rx) = mpsc::channel::<MqttMessage>(10);

    // Mock box_clone to return a new mock
    mock_listener
        .expect_box_clone()
        .returning(|| Box::new(MockDnsListener::new()));

    // Set up expectations for the mock listener
    mock_listener
        .expect_start_listening()
        .returning(move |_| Ok(()));

    // The bridge should send birth message on start
    mock_publisher.expect_send_birth().returning(|| Ok(()));

    // The bridge should send death message when finished
    mock_publisher.expect_send_death().returning(|| Ok(()));

    // Create a bridge with our mocks
    let bridge = Bridge::from_components(Box::new(mock_listener), Some(Box::new(mock_publisher)));

    // Use a timeout to stop the bridge after a short time
    let handle = tokio::spawn(async move {
        // This will run until a signal is received or it errors
        let _ = bridge.start().await;
    });

    // Give a little time for the bridge to start and then simulate a ctrl+c
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Send simulated Ctrl+C
    // Note: This won't work in a test environment, so we'll abort the task instead
    handle.abort();

    // Wait for the task to finish
    let _ = handle.await;
}

#[tokio::test]
async fn test_message_processing() {
    // Create mocks
    let mut mock_listener = MockDnsListener::new();
    let mut mock_publisher = MockMessagePublisher::new();

    // Shared message channel
    let (tx, _) = mpsc::channel::<MqttMessage>(10);
    let tx_clone = tx.clone();

    // Create a flag to track if publish was called
    let publish_called = Arc::new(Mutex::new(false));
    let publish_called_clone = Arc::clone(&publish_called);

    // Set up the mock listener to send a test message
    mock_listener.expect_box_clone().returning(move || {
        let mut mock = MockDnsListener::new();
        let _tx = tx_clone.clone();
        mock.expect_start_listening().returning(move |sender| {
            let sender_clone = sender.clone();
            // Send a test message
            tokio::spawn(async move {
                let message = MqttMessage::Publish {
                    topic: "test/topic".to_string(),
                    payload: b"test_payload".to_vec(),
                };
                let _ = sender_clone.send(message).await;
                // Give some time for processing
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            });
            Ok(())
        });
        Box::new(mock)
    });

    // The bridge should send birth message on start
    mock_publisher.expect_send_birth().returning(|| Ok(()));

    // Expect publish to be called with our test message
    mock_publisher.expect_publish().returning(move |_| {
        // Mark that publish was called
        let mut flag = publish_called_clone.lock().unwrap();
        *flag = true;
        Ok(())
    });

    // The bridge should send death message when finished
    mock_publisher.expect_send_death().returning(|| Ok(()));

    // Create a bridge with our mocks
    let bridge = Bridge::from_components(Box::new(mock_listener), Some(Box::new(mock_publisher)));

    // Run the bridge with a timeout
    let handle = tokio::spawn(async move {
        let _ = bridge.start().await;
    });

    // Give enough time for the message to be processed
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    // Check that publish was called
    let was_called = *publish_called.lock().unwrap();
    assert!(was_called, "Message publish should have been called");

    // Abort the bridge task
    handle.abort();
    let _ = handle.await;
}

#[tokio::test]
async fn test_bridge_without_mqtt() {
    // Create mocks
    let mut mock_listener = MockDnsListener::new();

    // Configure the mock listener
    mock_listener
        .expect_box_clone()
        .returning(|| Box::new(MockDnsListener::new()));

    mock_listener.expect_start_listening().returning(|_| Ok(()));

    // Create a bridge without MQTT
    let bridge = Bridge::from_components(Box::new(mock_listener), None);

    assert!(!bridge.has_mqtt_config());

    // Run the bridge with a timeout
    let handle = tokio::spawn(async move {
        let _ = bridge.start().await;
    });

    // Give a little time for the bridge to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Abort the bridge task
    handle.abort();
    let _ = handle.await;
}
