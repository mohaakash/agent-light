# Architecture Decisions and Open Questions

This file records decisions so that implementation does not silently drift.

## Accepted decisions

### ADR-001: Tauri with Rust core

**Decision:** Use Tauri 2 with React/TypeScript frontend and Rust backend. Do not add a Go helper for the initial product.

**Reason:** Tauri already uses Rust. One backend avoids a second runtime, packaging pipeline and local IPC boundary.

### ADR-002: Virtual-first product

**Decision:** The virtual traffic light is a first-class output and must work without ESP8266 hardware.

**Reason:** The software should be useful to users who do not build or purchase the hardware.

### ADR-003: Shared status engine

**Decision:** Virtual and physical outputs receive the same resolved display state.

**Reason:** Prevent inconsistent behavior and duplicated status logic.

### ADR-004: Local-first event transport

**Decision:** Start with an authenticated loopback HTTP API for agent adapters.

**Reason:** Easy interoperability with integrations written in different languages. Revisit IPC only if there is a concrete security or reliability benefit.

### ADR-005: ESP8266 USB serial for MVP

**Decision:** Use NodeMCU ESP8266 over USB serial. Wi-Fi/MQTT is not required for the initial prototype.

**Reason:** Simpler setup and no network dependency.

### ADR-006: Milestone 1 is a virtual prototype

**Decision:** The first implementation milestone is limited to a Tauri 2 + React + TypeScript desktop shell, a Rust-owned display state, and a separate virtual traffic-light window. Local HTTP ingress, session aggregation, serial hardware and agent integrations remain later phases.

**Reason:** A working virtual vertical slice validates the window behavior and shared output contract before adding external event sources or hardware.

### ADR-007: Two native windows, one frontend bundle

**Decision:** AgentLight uses one Tauri process with a `main` dashboard window and a `traffic-light` webview window. Both use the same React bundle and select their UI by window URL. Rust creates the windows and broadcasts display-state changes to both.

**Reason:** This preserves one application boundary while keeping the widget independently movable and always-on-top.

### ADR-008: Manual preview is not session state

**Decision:** Milestone 1 development controls invoke Rust commands that update a manual preview snapshot. They do not simulate agent sessions or imply permission approval. The session manager and resolver will replace this source in Phase 2 without changing the output window contract.

**Reason:** The UI must not become the authoritative state owner, and preview behavior must remain clearly separate from real agent lifecycle semantics.

### ADR-009: Native drag handling for the frameless widget

**Decision:** The traffic-light webview uses an explicit `startDragging()` call from its pointer handler, backed by the `core:window:allow-start-dragging` capability. The declarative `data-tauri-drag-region` attribute remains as a semantic marker, but is not the only drag mechanism.

**Reason:** A transparent, frameless Tauri window must initiate movement through the native window API. Handling the gesture on the widget root makes the entire indicator draggable, including its child light elements, and allows the OS to move it across monitor boundaries.

**Compatibility:** This changes only window interaction. The widget remains non-resizable and always-on-top behavior is unchanged. The native position is persisted in the application settings after drag completion and restored on launch with visible-monitor clamping.

### ADR-010: One real integration first

**Decision:** Implement one fully tested agent integration before expanding to all target agents.

**Reason:** Validate event semantics and installation flow before multiplying integration complexity.

### ADR-011: Lifecycle hooks over transcript polling

**Decision:** Agent integrations use documented lifecycle hooks to send normalized status metadata to AgentLight's authenticated loopback endpoint. AgentLight does not poll or parse agent transcripts. Codex, Claude Code, Cursor, Gemini CLI, Windsurf, GitHub Copilot CLI, and Antigravity adapters are implemented through dedicated or shared hook bridges.

**Reason:** Hooks expose lifecycle points without coupling AgentLight to transcript internals. Hooks send only the canonical event, session identifier, project basename and timestamp; they never send prompts, source code, tool arguments or terminal output.

**Compatibility:** The bridge is additive and failure-tolerant. If AgentLight is closed or unavailable, hook commands return without disrupting their agent. Multiple-session aggregation is resolved by the Rust session core.

## Open questions

These are intentionally unresolved. Do not silently invent a final product decision.

1. Which operating systems will receive validation beyond the current Windows development and packaging target?
2. Which additional coding agents should be prioritized after the implemented Codex, Claude Code, and shared adapters? OpenCode and Pi are current candidates.
3. Should the floating widget default to a classic three-light housing or a minimal three-dot design?
4. Should the widget support click-through mode, and how will users regain interaction?
5. Should closing the main window minimize to tray by default?
6. Should the local API token be stored in the OS credential vault, a protected config file, or another mechanism?
7. What completion display duration should ship by default? Provisional value: 5 seconds.
8. What stale-session timeout should ship by default? Provisional value: 2 minutes.
9. Should manual dismissal clear a waiting indicator, and how should the UI communicate that this does not approve the agent request?
10. Which OSes can be tested for the first public release?

## Change process

When a decision changes:
- Update this file.
- Update the affected specification.
- Explain migration or compatibility implications.
- Do not leave contradictory requirements in different documents.
