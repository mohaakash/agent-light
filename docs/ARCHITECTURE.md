# Architecture

## 1. Architectural approach

Use a **single Tauri application** with a Rust core and a React/TypeScript UI. Do not introduce a separate Go daemon for the initial design.

The Rust core owns domain state, event ingestion, session tracking, status resolution, serial communication and integration management. The frontend presents state and sends user commands.

The physical and virtual traffic lights are output providers. They must not implement their own independent status logic.

## 2. High-level components

```text
Coding agents
  | agent-specific hooks / plugins / extensions
  v
Local event ingress
  | authenticated localhost HTTP API (initial approach)
  v
Rust application core
  |-- Event validator and normalizer
  |-- Session manager
  |-- Status resolver
  |-- Integration manager
  |-- Configuration store
  |
  +--> Tauri event --> Virtual traffic-light window
  |
  +--> Serial manager --> ESP8266 firmware --> LEDs
  |
  +--> Tauri commands/events --> Main dashboard
```

## 3. Responsibility boundaries

### Frontend (React + TypeScript)

Responsible for:
- Main dashboard and settings UI.
- Floating virtual traffic-light rendering.
- User interaction: drag, hide/show, settings and manual test.
- Displaying session list, device status and integration status.
- Subscribing to Tauri events.

Not responsible for:
- Resolving aggregate agent status.
- Direct serial-port access.
- Parsing agent-specific hook payloads.
- Owning authoritative session state.

### Rust core

Responsible for:
- Canonical event types and validation.
- Session lifecycle and stale-session handling.
- Aggregate status resolution.
- Local API server lifecycle and authentication.
- Serial device discovery and communication.
- Configuration persistence.
- Integration installation checks and safe setup operations.
- Emitting state changes to Tauri windows.

### Agent adapters

Responsible for:
- Receiving the source agent's documented events.
- Translating source-specific events into the canonical event schema.
- Sending events to the local AgentLight ingress endpoint.
- Avoiding collection of prompts, source files or full tool output.

Adapters should be independently versioned/tested where practical. The core must not depend on an agent-specific SDK.

### ESP8266 firmware

Responsible for:
- Receiving a small, documented serial protocol.
- Validating commands.
- Driving LED outputs.
- Returning a simple acknowledgement or device identity response.
- Entering a safe state after communication timeout.

It must not know which coding agent produced an event.

## 4. Output provider model

Conceptually, the core publishes a resolved `DisplayState` to all enabled outputs.

```text
StatusResolver
    |
    v
DisplayState
    |
    +--> VirtualOutput --> Tauri event/window
    |
    +--> HardwareOutput --> SerialManager --> ESP8266
```

The virtual output is a Tauri window, not a separate process. The hardware output is optional.

A Rust trait may be used if it improves testability, but avoid unnecessary abstraction before a second or third output implementation exists. Keep the domain interface small.

## 5. Windows

### Main window

- Dashboard, sessions, hardware, integrations, settings and diagnostics.
- Normal decorated application window.
- Closing behavior should be configurable: quit or minimize to tray.

### Traffic-light window

- Separate Tauri webview window.
- Transparent background where supported.
- Frameless.
- Always-on-top option.
- Movable by dragging. The frameless widget starts native Tauri window dragging from its root pointer handler, with `core:window:allow-start-dragging` enabled.
- No taskbar entry where supported.
- Position and display selection persisted.
- Must not own the status engine.

In Milestone 1 the traffic-light window uses the same React bundle as the dashboard with a dedicated `traffic-light` window URL. Rust creates the native windows and broadcasts a serialized `display-state-changed` event to both windows. The dashboard's manual controls call Rust commands and do not maintain an authoritative copy of status.

### System tray

- Show/hide dashboard.
- Show/hide traffic light.
- Enable/disable outputs.
- Open settings.
- Quit application.

## 6. Event lifecycle

1. An adapter observes an agent event.
2. The adapter maps it to the canonical schema.
3. It posts the event to the local ingress endpoint.
4. The Rust ingress validates authentication, size, schema and timestamp.
5. The session manager updates the relevant session.
6. The status resolver recalculates aggregate display state.
7. The core emits a Tauri event to the UI.
8. The serial output sends the resolved state if hardware output is enabled.

## 7. Failure isolation

- Virtual output must continue when hardware is disconnected.
- Hardware output failure must not prevent event ingestion.
- One agent adapter failure must not prevent other adapters from working.
- Invalid events must be rejected and logged without crashing the app.
- If the UI window is closed, the core should continue only if the app's configured tray/background mode is active.
- If the app exits, the firmware timeout should eventually switch LEDs off.

## 8. Security boundaries

- Bind the local API to loopback only (`127.0.0.1`), not all interfaces.
- Generate a per-installation random local token.
- Do not place the token in command-line arguments if avoidable.
- Store secrets using an OS-appropriate secure store where feasible; never commit secrets.
- Validate request size, event type, session identifiers and timestamps.
- Do not expose a general shell-execution endpoint.
- Integration installation must only modify known agent configuration locations after user consent.
- Do not automatically approve agent permissions.
- Do not transmit telemetry by default.

## 9. Architecture constraints

- No cloud dependency for core operation.
- No Go helper process in the initial architecture.
- No direct frontend-to-serial communication.
- No agent-specific logic in the ESP8266 firmware.
- No assumption that all agents provide equivalent lifecycle events.
