# Technology Stack

## 1. Selected stack

| Layer | Choice | Responsibility |
|---|---|---|
| Desktop shell | Tauri 2 | Native windows, tray, packaging and frontend bridge |
| Frontend | React + TypeScript | Dashboard and virtual indicator |
| Styling | CSS | Lightweight styling and themes |
| Backend | Rust | Core application and domain logic |
| Async runtime | Tokio, if needed | Concurrent server and I/O tasks |
| Serialization | Serde | Typed JSON parsing and output |
| Local API | Rust HTTP server (select a maintained framework) | Agent event ingress |
| Serial | Rust `serialport` crate (4.10.1) | Cross-platform discovery and blocking USB serial I/O for the ESP8266 |
| Persistence | Tauri Store or SQLite | Preferences and local state |
| Logging | `tracing` | Diagnostics |
| Firmware | Arduino framework / C++ | ESP8266 LED control |
| Firmware tooling | PlatformIO or Arduino IDE | Build and flash firmware |
| Agent adapters | TypeScript, JavaScript, shell or Python as appropriate | Agent-specific event forwarding |

## 2. Rationale

### Rust instead of Go

Tauri already requires a Rust backend. Using Rust for the core avoids a second runtime, separate helper packaging and inter-process coordination between Go and Tauri. Rust also provides direct access to serial libraries and native desktop capabilities.

### React + TypeScript

React provides a familiar component model for the dashboard and floating widget. TypeScript makes event contracts easier to share and validate.

### ESP8266

The NodeMCU ESP8266 is sufficient for a three-LED USB-serial prototype. The first version does not require Wi-Fi, MQTT or a cloud service.

## 3. Dependency policy

- Verify current crate and package maintenance before adding dependencies.
- Prefer a small dependency set.
- Pin compatible versions through lockfiles.
- Avoid adding a large state-management library unless the UI complexity justifies it.
- Use typed request and event structures.
- Keep hardware-independent core logic testable without a physical device.
- Document platform-specific limitations.

## 4. Platform targets

Target Windows, macOS and Linux, but do not claim equal support until each is tested.

For each release, record:
- OS and version tested.
- CPU architecture.
- Installer/package type.
- Serial permissions and driver requirements.
- Floating-window behavior and known limitations.

## 5. Build outputs

- Desktop application installer/package for each supported OS.
- ESP8266 firmware binary or documented build instructions.
- Agent integration packages/configuration templates.
- Checksums and release notes.
