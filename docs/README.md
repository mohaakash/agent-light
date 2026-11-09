# AgentLight

**Physical and virtual status indicators for AI coding agents.**

AgentLight is a cross-platform desktop application that makes the activity of AI coding agents visible through:
- A draggable, always-on-top virtual traffic light.
- An optional physical three-LED traffic light powered by an ESP8266 NodeMCU.
- A unified event and session model for supported coding agents.

The desktop application must remain useful without the physical hardware.

## Documentation map

| File | Purpose |
|---|---|
| `PRODUCT_SPEC.md` | Product goals, users, requirements, scope and behavior |
| `ARCHITECTURE.md` | System architecture, boundaries and data flow |
| `TECH_STACK.md` | Languages, frameworks, libraries and rationale |
| `DESIGN.md` | UI/UX principles, windows and interaction behavior |
| `STATUS_MODEL.md` | Canonical statuses, event semantics and aggregation |
| `LOCAL_API.md` | Local integration API and security requirements |
| `HARDWARE_FIRMWARE.md` | ESP8266 wiring, firmware behavior and serial protocol |
| `AGENT_INTEGRATIONS.md` | Adapter strategy and per-agent integration requirements |
| `PROJECT_STRUCTURE.md` | Proposed repository layout and module ownership |
| `ROADMAP.md` | Phased MVP and release plan with acceptance criteria |
| `CODEX_GUIDE.md` | Instructions for AI coding agents working in this repository |
| `DECISIONS.md` | Architectural decisions and open questions |

## Product principles

1. **Virtual-first:** the app works fully without hardware.
2. **One status engine:** virtual and physical outputs consume the same resolved state.
3. **Agent-agnostic core:** agent-specific behavior belongs in adapters.
4. **Local-first:** no cloud account or internet connection is required for core functionality.
5. **Safe by default:** AgentLight reports permission requests; it never approves them.
6. **Incremental delivery:** establish a working vertical slice before adding every integration.

## Development setup

Prerequisites:
- Node.js and npm.
- Rust stable with the target desktop platform's Tauri prerequisites installed.

From the repository root:

```sh
npm install
npm run tauri dev
```

To create a frontend production build, run `npm run build`. On Windows, create MSI and NSIS installers with `npm run tauri build`.

## Current project status

The repository contains the virtual traffic-light prototype, Codex and Claude Code lifecycle-hook adapters, a Rust session resolver, a token-protected loopback event API, and an ESP8266 firmware/serial output path. The session core tracks independent `(agent, session_id)` sessions, resolves aggregate priority, ignores duplicate and out-of-order events, expires stale work, and shows completion for five seconds. API logs omit tokens and event bodies. The widget's always-on-top preference and screen position are saved in the application config directory and restored on launch. If a saved position is no longer on a connected monitor, it is moved back onto an available display.

The Windows app has MSI and NSIS installers, generated platform icons, a live session dashboard, persisted widget appearance, tray controls, optional launch-at-login, and CI checks that publish the installers as workflow artifacts. macOS and Linux launch behavior, multi-monitor edge cases, live Codex/Claude hook smoke tests, and physical ESP8266/LED behavior still need validation. Additional agent integrations and compact/expanded widget layouts remain future work.

Start with `CODEX_GUIDE.md`, then read the product, architecture, status model, and roadmap documents before coding.
