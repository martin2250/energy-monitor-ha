# STPM34 + ESP32 Energy Monitor

## Setup
- press button for 0.1s -> enable http configuration server (useful if already connected to wifi)
- press button for 1s -> enable access point (no PW) and config server (192.168.4.1)
- HTTP GET and POST config
  - `/config_mqtt.json`
  - `/config_wifi.json`
  - `/config_stpm.json`
  - `/config_calibration.json`
  - `/save` saves configuration to EEPROM

## LEDs
- green
  - off: not connected to wifi access point
  - blinking: connected to wifi, not MQTT
  - on: connected to MQTT server
- red
  - off: access point and config server disabled
  - blinking: config server enabled
  - on: access point enabled

## Hardware
requires a >32kbit< SOT-23 EEPROM, eg `AT24C32E`

## TODO:
- fix energy counter
- use + update zcr config from MQTT
