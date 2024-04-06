# STPM34 + ESP32 Energy Monitor

Hardware files are [here](https://github.com/DM2PF/EnergyMonitor32).

## Setup
- press button for 0.1s -> enable http configuration server (useful if already connected to wifi)
- press button for 1s -> enable access point (no PW) and config server (192.168.4.1)
- HTTP GET and POST config
  - `/config_mqtt.json`
  - `/config_wifi.json`
  - `/config_stpm.json`
  - `/config_calibration.json`
  - `/save` saves configuration to EEPROM

You can use wget / curl to configure the device:
- `wget http://100.124.102.101/config_calibration.json`
- `curl -d "@config_calibration.json" -X POST http://100.124.102.101/config_calibration.json`
- `curl -X POST http://100.124.102.101/save`

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
