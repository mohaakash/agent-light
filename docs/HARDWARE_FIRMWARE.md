# Hardware and Firmware Specification

## 1. MVP hardware

- NodeMCU ESP8266 development board.
- One red LED.
- One yellow LED.
- One green LED.
- Three 220–330 ohm resistors (one per LED).
- Breadboard or prototype board.
- Jumper wires.
- USB data cable.

The USB connection supplies board power and serial communication. Do not assume every USB cable carries data.

## 2. Suggested pin mapping

| LED | NodeMCU label | ESP8266 GPIO |
|---|---|---|
| Red | D1 | GPIO5 |
| Yellow | D2 | GPIO4 |
| Green | D5 | GPIO14 |

Suggested wiring:
- GPIO pin -> resistor -> LED anode (long leg).
- LED cathode (short leg) -> GND.
- All grounds share NodeMCU GND.
- Each LED must have its own current-limiting resistor.

Verify the exact board pinout before wiring. NodeMCU clones and board layouts can vary. Do not connect an LED directly without a resistor.

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
AGENTLIGHT:ESP8266:1
ACK:STATUS:WORKING
PONG
ERR:UNKNOWN_COMMAND
```

The firmware sends `AGENTLIGHT:ESP8266:1` after boot and in response to `HELLO`. The desktop retries the handshake because opening a USB serial port may reset the board. Every valid command refreshes the firmware watchdog. The desktop sends `PING` every five seconds while connected; firmware turns all LEDs off after 15 seconds without a valid command.

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

The PlatformIO project is in `firmware/esp8266`:

```sh
pio run -d firmware/esp8266
pio run -d firmware/esp8266 -t upload
```

Select the detected serial port in AgentLight after the board restarts. The dashboard's Scan and Connect controls identify the firmware before enabling LED tests. A serial disconnect marks the hardware output disconnected; virtual status continues independently. Reconnect is user initiated.

The desktop serial implementation uses the cross-platform `serialport` crate. This repository has compile-tested the Rust host code, but firmware flashing, LED wiring, timeout behavior and reconnection still require a NodeMCU and the specified LEDs/resistors.
