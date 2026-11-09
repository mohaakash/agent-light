# Product Specification

## 1. Overview

AgentLight is a cross-platform desktop companion for AI coding agents. It translates agent lifecycle events into simple visual signals.

It has two output modes:

- **Virtual traffic light:** a small, movable, always-on-top desktop window.
- **Physical traffic light:** an optional ESP8266 NodeMCU connected over USB serial and driving three LEDs.

Both outputs must display the same resolved status. Either output can be enabled independently.

## 2. Problem

When coding agents run in terminals or editor panels, users may not notice that an agent has finished, is waiting for permission, or needs attention. AgentLight provides a glanceable ambient indicator.

## 3. Target users

- Developers using one or more AI coding agents.
- Users who want a persistent desktop indicator while working in another application.
- Makers who want an optional physical desk accessory.
- Users who do not own hardware but want the virtual indicator.

## 4. Goals

- Display agent activity without requiring the agent UI to be focused.
- Support multiple sessions and multiple agent types.
- Provide a virtual indicator without hardware.
- Support an optional ESP8266-based physical indicator.
- Offer a simple local integration interface.
- Run on Windows, macOS and Linux, subject to platform-specific validation.
- Keep core operation offline and local.

## 5. Non-goals for the initial release

- Controlling or approving coding-agent actions.
- Reading source code, prompts, terminal output, or private project files.
- Sending telemetry to a cloud service.
- Building a coding agent or orchestrating agent tasks.
- Supporting every coding agent at launch.
- Custom PCB manufacturing or a commercial hardware enclosure.
- Complex multi-user networking.

## 6. Core user journeys

### 6.1 Virtual-only user

1. Install AgentLight.
2. Launch the app.
3. Enable the virtual traffic light.
4. Install or configure an agent integration.
5. Start a supported coding-agent session.
6. Observe status changes in the floating widget.
7. Drag the widget to a convenient location and adjust its size, opacity, and housing theme.

### 6.2 Hardware user

1. Connect the NodeMCU to the computer over USB.
2. Open AgentLight and select the detected serial device.
3. Run the LED test.
4. Enable physical output.
5. Use the virtual and physical indicators together or independently.

### 6.3 Multiple sessions

1. Start multiple supported agent sessions.
2. AgentLight tracks each session separately.
3. The dashboard shows per-session state.
4. The traffic light displays the aggregate state according to `STATUS_MODEL.md`.

## 7. Functional requirements

### P0 — MVP

- F-001: The desktop app launches without connected hardware.
- F-002: The app exposes a virtual traffic-light window.
- F-003: The virtual window can be shown, hidden and dragged.
- F-004: The virtual window supports always-on-top behavior.
- F-005: The virtual indicator renders the canonical statuses.
- F-006: A local event interface accepts normalized agent events.
- F-007: The Rust core tracks sessions and resolves aggregate status.
- F-008: The app can discover serial ports and connect to a selected ESP8266.
- F-009: The app can send status changes to the ESP8266.
- F-010: A manual LED test is available.
- F-011: Device disconnection is handled without crashing the app.
- F-012: The app provides useful logs and diagnostics.

### P1 — First integrations and usability

- F-101: Add one fully tested coding-agent integration.
- F-102: Add integration installation and removal guidance.
- F-103: Show connected device and integration status in the dashboard.
- F-104: Persist user preferences and widget position.
- F-105: Support startup-on-login where supported. (Implemented through Tauri autostart plugin.)
- F-106: Provide clear permission-request and error states.
- F-107: Support multiple sessions and deterministic aggregation.

### P2 — Expansion

- F-201: Add further agent adapters.
- F-202: Add widget size, opacity and theme settings. (Implemented.)
- F-203: Add compact and expanded virtual widget layouts.
- F-204: Add optional OLED or other hardware output in a future version.
- F-205: Add signed installers and release automation.

## 8. Status semantics

The initial user-facing states are:

| State | Meaning | Indicator |
|---|---|---|
| `WAITING` | Human attention or permission is required | Red |
| `WORKING` | Agent is actively processing | Yellow |
| `COMPLETED` | A tracked task/session has completed | Green |
| `IDLE` | No active work or attention request | All off |
| `ERROR` | Agent or integration reported an error | Red, distinguish from WAITING in UI |

The exact event-to-state mapping is agent-specific and must be documented per adapter. Do not assume every agent exposes every state.

## 9. Reliability and usability requirements

- A stale session must not remain active forever.
- Device loss must not affect the virtual output.
- Virtual output must work when serial support is unavailable.
- A single adapter failure must not crash the core.
- The app must make it clear when an indicator is disconnected or stale.
- The app must never imply that a permission request was approved.
- The app must not collect agent prompts or source code.

## 10. MVP definition of done

The MVP is complete when:
1. The app launches on the primary development OS without hardware.
2. The virtual traffic light can be moved and kept above other windows.
3. A test event changes its status.
4. A local client can submit a valid event.
5. Rust resolves the event and emits a status update.
6. If an ESP8266 is connected, the same status is sent to its LEDs.
7. Disconnecting the ESP8266 does not break virtual mode.
8. Basic setup and troubleshooting instructions exist.
