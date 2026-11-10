# AgentLight

AgentLight is a local desktop companion that turns AI coding-agent activity into physical and virtual status indicators.

![AgentLight demo](docs/assets/agentlight-demo-optimized.gif)

## Features

- A draggable, always-on-top virtual traffic-light widget for each configured agent.
- A Rust session core that aggregates multiple agent sessions deterministically.
- A token-protected loopback API for lifecycle events.
- Agent adapters for OpenAI Codex, Claude Code, Cursor, Gemini CLI, Windsurf, GitHub Copilot CLI, and Antigravity.
- Optional USB-serial hardware output with firmware projects for ESP8266, ESP32, Raspberry Pi Pico, and Arduino.
- Hardware-independent operation: the virtual indicator works without a connected device.

AgentLight sends status metadata only. It does not read prompts, source code, terminal output, or tool arguments, and it never approves agent permissions.

## Development setup

Prerequisites:

- Node.js and npm.
- Rust stable and the Tauri prerequisites for your operating system.

From the repository root:

```sh
npm install
npm run tauri dev
```

Useful checks and builds:

```sh
npm run lint
npm run test:integrations
npm run build
npm run tauri build
```

The current CI and packaging target is Windows. `npm run tauri build` produces the configured MSI and NSIS installers on Windows.

## Agent integrations

Start AgentLight once before installing an integration so it can create the local API bridge configuration.

Codex:

```powershell
node integrations/codex/install.mjs
```

Claude Code:

```powershell
node integrations/claude/install.mjs
```

Cursor, Gemini CLI, Windsurf, GitHub Copilot CLI, or Antigravity:

```powershell
node integrations/agents/install.mjs <agent>
```

Replace `<agent>` with `cursor`, `gemini`, `windsurf`, `copilot`, or `antigravity`. Each installer preserves the agent's existing configuration and has a matching uninstall command.

## Documentation

The complete product and implementation documentation is in [`docs/README.md`](docs/README.md). Important references include:

- [`docs/ROADMAP.md`](docs/ROADMAP.md) — implementation status and remaining work.
- [`docs/AGENT_INTEGRATIONS.md`](docs/AGENT_INTEGRATIONS.md) — adapter strategy and lifecycle mappings.
- [`docs/LOCAL_API.md`](docs/LOCAL_API.md) — local event API and security model.
- [`docs/HARDWARE_FIRMWARE.md`](docs/HARDWARE_FIRMWARE.md) — wiring, firmware, and serial protocol.
- [`docs/CODEX_GUIDE.md`](docs/CODEX_GUIDE.md) — repository guidance for coding agents.

## Current validation status

The Windows desktop build, session core, local API tests, adapter installer tests, and firmware projects are in the repository. Live smoke tests with installed agents, physical LED verification, cross-platform packaging, and mixed-DPI multi-monitor behavior still need validation.
