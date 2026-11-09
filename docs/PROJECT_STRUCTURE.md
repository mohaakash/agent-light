# Proposed Project Structure

This is a target structure, not a requirement to create every directory immediately. The Milestone 1 implementation starts with the `app`, `windows/traffic-light`, `lib`, `core`, `commands` and `outputs` areas. Add `events`, `sessions`, `hardware`, `api`, `integrations`, `config` and `diagnostics` when their corresponding features arrive.

```text
agent-light/
├── package.json
├── index.html
├── src/
│   ├── app/                     # React dashboard
│   ├── components/
│   ├── features/
│   │   └── overview/
│   ├── windows/
│   │   └── traffic-light/       # Floating widget UI
│   ├── lib/                     # Frontend bridge and shared types
│   └── styles/
│
├── src-tauri/
│   ├── src/
│   │   ├── lib.rs
│   │   ├── main.rs
│   │   ├── commands/
│   │   ├── core/
│   │   └── outputs/
│   └── Cargo.toml
│
├── firmware/
│   └── esp8266/
│       ├── platformio.ini
│       └── src/
│           └── main.cpp
│
├── integrations/
│   ├── codex/
│   ├── claude/
│   ├── opencode/
│   ├── pi/
│   └── antigravity/
│
├── docs/
│   ├── README.md
│   ├── PRODUCT_SPEC.md
│   ├── ARCHITECTURE.md
│   ├── TECH_STACK.md
│   ├── DESIGN.md
│   ├── STATUS_MODEL.md
│   ├── LOCAL_API.md
│   ├── HARDWARE_FIRMWARE.md
│   ├── AGENT_INTEGRATIONS.md
│   ├── PROJECT_STRUCTURE.md
│   ├── ROADMAP.md
│   ├── CODEX_GUIDE.md
│   ├── DECISIONS.md
│   ├── development/
│   ├── testing/
│   └── releases/
│
└── tests/
    ├── fixtures/
    └── integration/
```

## Module ownership

- `core`: canonical domain types and application orchestration.
- `events`: validation, normalization and event dispatch.
- `sessions`: per-session state, stale handling and aggregation input.
- `outputs`: output abstraction and fan-out.
- `hardware`: serial discovery, protocol and device lifecycle.
- `api`: loopback HTTP server and authentication.
- `integrations`: adapter metadata and installation status.
- `config`: settings and persistence.
- `diagnostics`: structured logs and health information.

Keep module boundaries practical. Avoid creating empty abstractions or excessive files before they are needed.
