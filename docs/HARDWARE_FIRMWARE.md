# Hardware and Firmware Specification

## 1. Supported USB serial hardware

- A compatible USB-serial microcontroller: ESP8266, ESP32, Arduino Uno/Nano, or Raspberry Pi Pico.
- One red LED.
- One yellow LED.
- One green LED.
- Three 220–330 ohm resistors (one per LED).
- Breadboard or prototype board.
- Jumper wires.
- USB data cable.

The USB connection supplies board power and serial communication. Do not assume every USB cable carries data.

## 2. Suggested pin mappings

| Board | Red | Yellow | Green |
|---|---:|---:|---:|
| NodeMCU ESP8266 | D1 / GPIO5 | D2 / GPIO4 | D5 / GPIO14 |
| ESP32 DevKit V1 | GPIO25 | GPIO26 | GPIO27 |
| Arduino Uno/Nano | D5 | D6 | D7 |
| Raspberry Pi Pico | GP6 | GP7 | GP8 |

Suggested wiring:
- GPIO pin -> resistor -> LED anode (long leg).
- LED cathode (short leg) -> GND.
- All grounds share the board GND pin.
- Each LED must have its own current-limiting resistor.

Verify the exact board pinout before wiring. Board layouts can vary. Do not connect an LED directly without a resistor.

## 3. Firmware responsibilities

- Configure LED pins as outputs.
- Start serial communication at 115200 baud.
- Start with all LEDs off.
- Parse newline-terminated commands.
- Validate commands and ignore malformed input.
- Acknowledge valid commands.
- Turn all LEDs off after communication timeout.
- Provide a device identity/version response.
- Avoid blocking delays that prevent serial processing.

## 4. Serial protocol

Initial text protocol, one command per line:

```text
HELLO
STATUS:WAITING
STATUS:WORKING
STATUS:COMPLETED
STATUS:IDLE
PING
```

Suggested responses:

```text
AGENTLIGHT:SERIAL:1
ACK:STATUS:WORKING
PONG
ERR:UNKNOWN_COMMAND
```

The firmware sends `AGENTLIGHT:SERIAL:1` after boot and in response to `HELLO`. The desktop also accepts the legacy `AGENTLIGHT:ESP8266:1` identifier. The desktop retries the handshake because opening a USB serial port may reset the board. Every valid command refreshes the firmware watchdog. The desktop sends `PING` every five seconds while connected; firmware turns all LEDs off after 15 seconds without a valid command.

The protocol should be case-sensitive and documented. Use a bounded input buffer and reject overlong lines.

## 5. Status-to-LED mapping

| Command | Red | Yellow | Green |
|---|---:|---:|---:|
| `WAITING` | ON | OFF | OFF |
| `WORKING` | OFF | ON | OFF |
| `COMPLETED` | OFF | OFF | ON |
| `IDLE` | OFF | OFF | OFF |
| `ERROR` | ON | OFF | OFF |

The virtual UI may distinguish WAITING and ERROR with labels or animation; the simple three-LED hardware mapping uses red for both.

## 6. Connection behavior

- Desktop discovers serial ports and lets the user select the device.
- Do not assume a fixed COM port or `/dev/ttyUSB*` path.
- On disconnect, mark hardware output disconnected.
- Keep virtual output functioning.
- Attempt reconnection only through a controlled strategy.
- On firmware timeout, switch all LEDs off.
- Do not let repeated reconnect attempts freeze the UI.

## 7. MVP validation

- Verify each LED independently.
- Verify each status command.
- Verify malformed commands do not change output.
- Verify unplugging USB does not crash the desktop app.
- Verify firmware timeout turns all LEDs off.
- Verify the same resolved status reaches virtual and physical outputs.

## 8. Firmware build and upload

PlatformIO projects are supplied for ESP8266, ESP32, and Raspberry Pi Pico:

```sh
pio run -d firmware/esp8266
pio run -d firmware/esp8266 -t upload
pio run -d firmware/esp32
pio run -d firmware/pico
```

For Arduino Uno/Nano, open `firmware/arduino/agentlight_traffic_light.ino` in the Arduino IDE, select the matching board and port, then upload. All examples use the same USB serial protocol and can be adapted to another Arduino-compatible board by changing the three pin numbers.

Select the detected serial port in AgentLight after the board restarts. The dashboard's Scan and Connect controls identify compatible AgentLight firmware before enabling LED tests. A serial disconnect marks the hardware output disconnected; virtual status continues independently. Reconnect is user initiated.

The desktop serial implementation uses the cross-platform `serialport` crate. This repository has compile-tested the Rust host code, but physical flashing, LED wiring, timeout behavior and reconnection still need validation on each listed board.
