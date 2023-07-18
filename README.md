# ring-detector

### ***NOTE***

*Currently this is just a proof-of-concept and doesn't send the right
kind of message via MQTT!*

A utility to detect when an EZVIZ DB1 doorbell is pressed and sends an MQTT
message to Home Assistant. It utilizes [dnstap](https://dnstap.info/)
functionality in a local LAN DNS server to identify when the doorbell is
pressed by watching for the doorbell doing a DNS lookup for a few domain
names.

