# Instructions for Codex and Other Coding Agents

You are working on **AgentLight**, a cross-platform desktop companion that displays AI coding-agent status through a virtual traffic light and optional ESP8266 hardware.

These documents are the project source of truth. Read this file first, then read the relevant specification files before making changes.

## 1. Required reading

Before implementation, read:
1. `README.md`
2. `PRODUCT_SPEC.md`
3. `ARCHITECTURE.md`
4. `STATUS_MODEL.md`
5. `TECH_STACK.md`
6. `ROADMAP.md`
7. The relevant component specification for the task.

## 2. Core architectural rules

- Use Tauri 2, React, TypeScript and Rust.
- Do not introduce a separate Go helper/daemon unless the user explicitly changes the architecture.
- Rust owns canonical event state, session management, status resolution, local API and serial communication.
- The virtual widget and ESP8266 are outputs of the same resolved status.
- The app must work without hardware.
- Agent-specific logic belongs in adapters, not in the status engine or firmware.
- Firmware must not know about coding-agent brands.
- Do not make the UI the source of truth for session state.

## 3. Working method

- Work in small, reviewable milestones.
- Before coding, inspect the existing repository and identify what is already implemented.
- Do not overwrite existing user work.
- Propose a short plan for non-trivial changes.
- Implement only the requested phase/task; do not opportunistically build the entire roadmap.
- Run relevant formatting, linting, tests and builds.
- Report commands run and their results accurately.
- If a command cannot be run, say why.
- Update documentation when behavior or architecture changes.

## 4. Engineering requirements

- Prefer typed Rust structures and explicit error handling.
- Keep domain logic independent of Tauri where practical so it can be unit-tested.
- Keep serial I/O non-blocking from the UI's perspective.
- Handle disconnects and malformed input.
- Validate all local API requests.
- Bind local HTTP only to `127.0.0.1`.
- Never log tokens, prompts, source code or sensitive agent payloads.
- Do not auto-approve agent permission requests.
- Do not blindly overwrite user agent configuration.
- Do not claim an integration is supported until tested against a documented agent version.

## 5. Dependency discipline

- Check current official documentation before selecting APIs or adding dependencies.
- Prefer maintained packages and minimal dependencies.
- Do not guess Tauri APIs, agent hook names or configuration formats.
- If external documentation is unavailable, clearly mark assumptions and ask before relying on them.
- Keep lockfiles committed where appropriate.

## 6. Hardware discipline

- Verify the exact NodeMCU pinout.
- Each LED must have its own current-limiting resistor.
- Treat USB serial device names as dynamic.
- Implement a firmware communication timeout that turns LEDs off.
- Keep virtual output operational when hardware is disconnected.
- Provide a mock serial/device layer for tests where useful.

## 7. UI discipline

- Keep the floating widget compact and unobtrusive.
- Support dragging, always-on-top and show/hide behavior.
- Persist position safely and recover if the previous monitor is unavailable.
- Do not make color the only status signal in the dashboard.
- Test platform-specific transparent-window behavior rather than assuming it is identical everywhere.

## 8. Definition of a completed task

A task is not complete merely because code was written. Completion requires:
- Implementation consistent with the specs.
- Relevant tests.
- Formatting/linting/build checks where available.
- Documentation updates where needed.
- A concise summary of changed files, behavior, tests and remaining limitations.

## 9. First task

When starting from an empty repository:
1. Inspect the environment and available toolchains.
2. Scaffold a minimal Tauri 2 + React + TypeScript application.
3. Confirm it builds and launches.
4. Create the separate virtual traffic-light window.
5. Implement local development controls to switch among `WAITING`, `WORKING`, `COMPLETED` and `IDLE`.
6. Do not implement agent integrations or ESP8266 serial support until the virtual prototype is working.

Before starting, summarize the plan and ask for confirmation if the requested task is broader than this first milestone.
