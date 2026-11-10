# Development Roadmap

## Guiding rule

Build a working vertical slice before expanding breadth. Do not attempt all agent integrations, installer polish and advanced themes in the first milestone.

## Phase 0 — Repository and development setup

Implementation status: Tauri 2 + React + TypeScript project is set up. Formatting, linting, and Windows CI checks are configured. Windows development and production builds are available.

Tasks:
- Create Tauri 2 + React + TypeScript project.
- Confirm Rust and frontend toolchains.
- Add formatting and linting.
- Add basic CI checks.
- Preserve this specification in the repository.

Acceptance:
- App launches on the primary development OS.
- README includes reproducible setup instructions.
- No hardware is required to run the app.

## Phase 1 — Virtual traffic-light prototype

Implementation status: complete for the documented Windows prototype, including persistent always-on-top preference and widget position. Cross-platform and multi-monitor/DPI validation remain open.

Tasks:
- Establish the minimal application structure described in `PROJECT_STRUCTURE.md`.
- Create separate traffic-light window.
- Render red/yellow/green/off states.
- Add show/hide and drag.
- Add always-on-top behavior.
- Route development state controls through the Rust core and broadcast one display snapshot to both windows.
- Add configuration-backed persistence for widget position and preferences. (Implemented.)
- Add a development-only test control.
- Handle transparent/frameless window behavior.

The prototype bundle was initially disabled. Windows MSI and NSIS packaging is now configured with generated icons and release metadata.

Acceptance:
- Widget can be moved.
- Widget stays above normal windows when enabled.
- Status changes update the widget.
- Main window can be closed/hidden without corrupting state.
- Document OS-specific limitations.

## Phase 2 — Canonical event and session core

Implementation status: complete. The Rust core tracks independent sessions, validates normalized agent event envelopes, resolves aggregate priority, deduplicates IDs, ignores stale valid timestamps, expires stale work, and applies the five-second completion window. Unit tests cover event ordering and multi-session status priority.

Tasks:
- Define typed event and status models.
- Implement event validation.
- Implement session tracking.
- Implement aggregate status priority.
- Implement completion display duration.
- Implement stale-session handling.
- Add unit tests for event ordering and multiple sessions.

Acceptance:
- Deterministic tests cover every status.
- Waiting takes priority over working.
- Completed is transient and then becomes idle.
- Stale work is not shown as completed.

## Phase 3 — Local API

Implementation status: complete for the current Codex, Claude Code, and shared agent adapters. The loopback server exposes authenticated event ingress and a minimal health endpoint, emits structured JSON logs without event bodies or tokens, and has route, request-reader, and loopback TCP tests.

Tasks:
- Implement loopback HTTP ingress.
- Add per-installation token.
- Validate payloads and body size.
- Add health endpoint.
- Add structured logs.
- Add API integration tests.

Acceptance:
- Valid authenticated event changes status.
- Missing/invalid token is rejected.
- Malformed and oversized payloads are rejected.
- API binds only to loopback.
- No sensitive agent content is stored.

## Phase 4 — ESP8266 hardware output

Implementation status: host-side serial discovery, connect/disconnect, device identity handshake, status forwarding, LED test controls and heartbeat handling are implemented. Firmware projects and PlatformIO configurations are present for ESP8266, ESP32, and Raspberry Pi Pico, with an Arduino sketch for compatible boards. Physical flashing, LED verification, unplug/reconnect testing and timeout validation remain open until tested with hardware.

Tasks:
- Build and flash firmware.
- Implement serial protocol and acknowledgement.
- Implement device discovery and connection.
- Add LED test controls.
- Add heartbeat/timeout safety.
- Connect hardware output to resolved display state.

Acceptance:
- All LEDs map correctly.
- Device disconnect does not affect virtual output.
- Reconnect works without restarting the app.
- Firmware timeout switches LEDs off.

## Phase 5 — Real agent integrations

Implementation status: Codex, Claude Code, Cursor, Gemini CLI, Windsurf, GitHub Copilot CLI, and Antigravity lifecycle mappings, multi-session resolution, idempotent setup/uninstall, process-level hook tests, and privacy checks are implemented. Manual smoke tests in live agent sessions remain outstanding.

Tasks:
- Select agents based on installed availability and documented event support.
- Implement adapters.
- Add setup and uninstall flow.
- Test start, work, permission, completion and error where available.
- Document unsupported signals.

Acceptance:
- Real agent events update the virtual indicator.
- Hardware mirrors the same resolved state when connected.
- Adapter does not disrupt agent operation.
- No prompt/source/tool payload is transmitted.

## Phase 6 — Multi-agent and product polish

Implementation status: live per-session dashboard, per-agent widget controls, persisted widget size/theme/opacity, system-tray controls, optional launch-at-login, Windows MSI/NSIS packaging, and CI artifact upload are implemented. Physical device verification is deferred by the user. OpenCode and Pi adapters, macOS/Linux packaging, live agent smoke testing, and multi-monitor/DPI validation remain open.

Tasks:
- Add remaining adapters, beginning with OpenCode and Pi when their supported lifecycle interfaces are documented.
- Improve session dashboard.
- Add appearance settings.
- Add tray behavior and startup options.
- Improve multi-monitor and DPI behavior.
- Package and test on target OSes.

Acceptance:
- Multiple sessions aggregate deterministically.
- User can enable/disable integrations.
- Preferences persist.
- Release artifacts and troubleshooting docs are available.

## Priority order

1. Virtual window.
2. Canonical status engine.
3. Local API.
4. Physical hardware output.
5. First real agent integration.
6. Session dashboard and Windows installer artifacts (implemented).
7. OpenCode/Pi integrations and compact/expanded widget layouts.
8. Live agent smoke tests, cross-platform packaging, and multi-monitor/DPI validation.

This order keeps the app useful from the first milestone and reduces the risk of debugging hardware, agent hooks and UI simultaneously.
