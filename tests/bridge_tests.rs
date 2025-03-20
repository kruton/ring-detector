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
