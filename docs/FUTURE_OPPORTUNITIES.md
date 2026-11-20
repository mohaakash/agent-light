# Future Opportunities & Implementation Specifications

This document details high-value architectural enhancements and feature opportunities identified for **AgentLight**. It provides deep implementation designs, trade-offs, and specifications so they can be executed systematically in upcoming milestones.

---

## Table of Contents

1. [One-Click In-App Hook Management & Native Diagnostics](#1-one-click-in-app-hook-management--native-diagnostics)
2. [Audio & Ambient Notifications on Attention](#2-audio--ambient-notifications-on-attention)
3. [Actionable Widget Interaction: Click-to-Focus & Agent Navigation](#3-actionable-widget-interaction-click-to-focus--agent-navigation)
4. [Multi-Monitor, Multi-DPI, and Mixed-Scaling Precision](#4-multi-monitor-multi-dpi-and-mixed-scaling-precision)
5. [Implementation Roadmap & Phasing](#5-implementation-roadmap--phasing)

---

## 1. One-Click In-App Hook Management & Native Diagnostics

### Problem & Motivation

Currently, connecting coding agents (Codex, Claude Code, Cursor, Gemini CLI, Windsurf, Copilot CLI, Antigravity) requires running Node.js scripts via the terminal (`node integrations/agents/install.mjs <agent>`) or through backend commands in [commands/integrations.rs](file:///C:/Users/Akash/projects/agent-light/src-tauri/src/commands/integrations.rs).

In production desktop releases (MSI, NSIS, DMG, AppImage), invoking Node.js via external processes has critical limitations:
1. **Runtime Dependency:** User environments might not have `node` in their GUI application PATH.
2. **Packaging Paths:** Development paths like `env!("CARGO_MANIFEST_DIR")` do not exist in installed applications.
3. **No In-App Rollback:** Uninstalling hooks requires manually hunting down configuration files or running separate CLI uninstall scripts.
4. **Lack of Diagnostics:** Users have no immediate visual feedback verifying that loopback events are actively reaching AgentLight.

### Proposed Architecture

Move from external Node.js installer scripts to **native Rust-powered configuration managers** combined with an interactive **Integration Center** in the UI.

```text
Dashboard UI (Agents Tab)
    │
    ├── Install / Uninstall / Revert
    ├── Live Health Status & Diagnostics
    └── Send Test Ping
           │
           ▼
Tauri Commands (Rust Core)
    │
    ├── Config Parser / Writer (serde_json / toml)
    │     ├── Direct atomic updates to agent config files
    │     └── Automatic timestamped backup files (.agentlight.bak)
    │
    └── Bridge Status Validator
          └── Loopback healthcheck & token validation
```

### Key Technical Specifications

1. **Native Config Modification in Rust:**
   * **Codex:** Read and write `~/.codex/hooks.json` and `~/.codex/config.toml` directly using `serde_json` and `toml`.
   * **Claude Code:** Read and write `~/.claude/config.json`.
   * **Cursor / Windsurf / Antigravity:** Read and write global rules/hooks configurations in user application data directories (`com.agentlight.app`).
   * **Safety First:** Create atomic backups (e.g., `hooks.json.agentlight.bak`) before touching any file. Never overwrite unrecognized configurations.

2. **Full Lifecycle UI Controls:**
   * **Status Badges:** `Not Installed`, `Installed & Healthy`, `Misconfigured`, `Out of Sync`.
   * **One-Click Actions:**
     * `Install Hook`: Injects the hook command without touching existing user hooks.
     * `Uninstall Hook`: Selectively removes the AgentLight hook while preserving other handlers.
     * `Verify Bridge`: Checks that the loopback server is reachable and the authentication token in `codex-bridge.json` matches the runtime secret.
     * `Send Test Signal`: Simulates a `permission_required` and `task_completed` event to verify widget responsiveness.

### Implementation Status

Completed. `src-tauri/src/integrations/hooks.rs` now performs all hook management natively in Rust (JSON via `serde_json`, atomic writes, `.agentlight-backup` snapshots before the first change) for Codex, Claude Code, Cursor, Gemini CLI, Windsurf, Copilot CLI, and Antigravity, including the Codex `[features] hooks = true` flag in `config.toml`. The Agents tab is an Integration Center with health badges (`Not installed`, `Installed & healthy`, `Installed (bridge offline)`, `Misconfigured`, `Out of sync`), install/uninstall, **Restore backup**, **Verify bridge** (loopback health check plus token comparison against the runtime secret), and test signals. Hook scripts are bundled as Tauri resources and resolved from packaged locations, so the desktop app no longer shells out to Node.js for configuration. The `integrations/*/install.mjs` scripts remain available for CLI use only.

---

## 2. Audio & Ambient Notifications on Attention

### Problem & Motivation

The visual traffic light is designed to be ambient, but developers frequently look away: working in a full-screen IDE on another display, reading documentation, or stepping away from the desk.

When an autonomous agent hits a blocking permission gate (e.g., running a terminal command, editing an out-of-scope file, or encountering an unhandled exception), it enters the `WAITING` or `ERROR` state (🔴 **Red**). If the visual indicator goes unnoticed, the agent sits idle, defeating the purpose of autonomous execution.

### Proposed Architecture

Introduce an optional, non-intrusive sound and notification system triggered only on state transitions that require developer intervention.

```text
Status Resolver / Session Manager
    │ State transition detected: * -> WAITING or ERROR
    ▼
Display State Broadcast ("display-state-changed")
    │
    ├── Virtual Widgets (Visual update: Red)
    ├── Serial Hardware (LED update: Red)
    └── Notification & Audio Dispatcher
           ├── Web Audio API / Rust Audio Engine (Subtle chime)
           └── OS System Notification (Tauri Notification Plugin)
```

### Key Technical Specifications

1. **Audio Synthesis & Presets:**
   * Implement via the **Web Audio API** in the frontend or natively in Rust using a lightweight audio backend (e.g., `rodio`).
   * Synthesize audio frequencies or bundle minimal, low-latency audio assets:
     * *Gentle Chime* (Default): Soft two-tone chime for `WAITING` (permission prompts).
     * *Alert Knock*: Distinct subtle knock for `ERROR`.
     * *Completion Tone* (Optional): Quiet affirmative chord for `COMPLETED`.
   * Provide a volume slider (0–100%) and a quick mute toggle directly in the dashboard and system tray menu.

2. **OS Desktop Notifications (Toasts):**
   * Integrate `@tauri-apps/plugin-notification`.
   * Fire system toast notifications when:
     * An agent enters `WAITING` while AgentLight is minimized or running in the tray.
     * Content: `"[Agent Name] in [Project] requires attention to proceed."`
   * Include "Do Not Disturb" (DND) modes or suppression when the agent's IDE window is already focused.

### Implementation Status

Completed. Transition-based attention/completion audio alerts, synthesized chime/knock/ping presets, a volume slider and preview, optional completion tones, saved mute/notification preferences, a system-tray sound toggle, and Tauri desktop notifications with permission handling. Notifications include the agent and project when available.

Do Not Disturb is implemented with an indefinite switch, timed snoozes (15 minutes, 1 hour, 4 hours) that expire automatically, and a tray toggle; while active it silences both sounds and desktop toasts. Foreground suppression skips toasts when the AgentLight dashboard is already focused, and the `Quiet while AgentLight is focused` option can be turned off. IDE-window-level focus detection is not implemented because the adapters do not yet report the hosting application.

---

## 3. Actionable Widget Interaction: Click-to-Focus & Agent Navigation

### Problem & Motivation

Developers often run multiple agents simultaneously across different projects (e.g., Claude Code in a backend terminal, Cursor in a frontend app, and Antigravity running a CLI workflow). When a red light appears, the developer knows *that* an agent is waiting, but still has to manually locate and switch to the relevant window, terminal, or IDE worktree.

### Proposed Architecture

Turn the virtual traffic light from a passive display into an **actionable navigation bridge**. Clicking a red indicator immediately brings the requesting agent's window or terminal to the foreground.

```text
User clicks Red Light on Widget
    │
    ▼
Tauri Command: focus_agent_session(agent_id, session_id)
    │
    ├── 1. Query SessionManager for session metadata (PID, CWD, IDE)
    ├── 2. Platform Window Manager:
    │      ├── Windows: Win32 SetForegroundWindow / BringWindowToTop
    │      ├── macOS: NSWorkspace / NSRunningApplication.activate
    │      └── Linux: xdotool / wmctrl / Wayland D-Bus activation
    └── 3. Fallback: Open project directory in default IDE/Terminal
```

### Key Technical Specifications

1. **Enriched Adapter Metadata:**
   * Extend the canonical hook payload to capture available context:
     * `process_id` (PID of the CLI process or IDE host).
     * `cwd` (Working directory / project root).
     * `client_app` (e.g., `"vscode"`, `"cursor"`, `"alacritty"`, `"iterm2"`, `"windows-terminal"`).

2. **OS Window Activation:**
   * **Windows (`winapi` / `windows-sys` crate):**
     * Use `EnumWindows` and `GetWindowThreadProcessId` to locate the window associated with the agent's PID or its parent terminal PID.
     * Call `AttachThreadInput` and `SetForegroundWindow` to bring the window cleanly to the foreground.
   * **macOS:**
     * Use `NSRunningApplication(processIdentifier: pid).activate(options: .activateIgnoringOtherApps)`.
   * **Graceful Fallback:** If OS-level process focus fails (or on restricted environments), trigger a system action opening the project's `cwd` in the default editor or terminal.

3. **Contextual Tooltip & Action Menu:**
   * Hovering over the red light displays a quick popover:
     * `Agent: Claude Code`
     * `Project: agent-light`
     * `State: Permission Required`
     * `[Click to focus window]`

### Implementation Status

Completed. Hook payloads now carry `cwd`, `client_app` (detected from `CURSOR_*`, `VSCODE_*`, `WINDSURF_*`, `TERM_PROGRAM`, and `WT_SESSION`), and `process_id` (the hook's parent process, which is the agent CLI). The core stores this metadata per session and exposes `get_agent_focus_target`, which returns the most urgent session for an agent. `focus_agent_session` restores and raises the matching window with `EnumWindows`/`GetWindowThreadProcessId`/`AttachThreadInput`/`SetForegroundWindow` on Windows, `osascript` on macOS, and `xdotool` on Linux, then falls back to opening the session's `cwd` in the OS file manager; failures return an actionable message. Traffic-light widgets distinguish a click from a drag (a pointer movement over 4px starts the native window drag), so clicking a widget focuses the agent, and hovering shows an agent/project/state popover with a click hint. Session rows in the dashboard gained a **Focus** button. Session metadata survives state updates because the manager merges fields rather than replacing them.

---

## 4. Multi-Monitor, Multi-DPI, and Mixed-Scaling Precision

### Problem & Motivation

In modern multi-monitor developer environments, displays frequently vary in resolution, aspect ratio, orientation, and DPI scaling factor:
* Primary display: 4K (3840×2160) at 150% scaling.
* Secondary display: 1080p (1920×1080) at 100% scaling.
* Vertical display: 1440×2560 at 125% scaling with negative desktop coordinates.

This causes critical edge cases in both **always-on-top dragging** and the new **Hotzone Mode**:
1. **Physical vs. Logical Coordinate Mismatches:** Tauri's `app.cursor_position()` returns physical screen pixels, whereas webview layout and window positioning use logical pixels.
2. **Negative Coordinate Spaces:** Displays positioned to the left or above the primary monitor have negative origin offsets (`x < 0` or `y < 0`), which break naive bounding-box collision checks.
3. **Corner Inaccuracies:** A 100px corner hotzone on a 4K 150% screen covers a physically much smaller surface area than on a 1080p 100% display, making hover detection feel inconsistent.

### Proposed Architecture

Implement a unified, monitor-relative coordinate normalization pipeline in the Rust core.

```text
Raw Global Pointer (Physical Pixels: x, y)
    │
    ▼
Monitor Identification (get_monitor_at)
    │ Match (x, y) within monitor physical bounding box:
    │   [mon.x, mon.x + mon.width] × [mon.y, mon.y + mon.height]
    ▼
Relative Coordinate Normalization
    │ local_x = (x - mon.x) / mon.scale_factor
    │ local_y = (y - mon.y) / mon.scale_factor
    ▼
Resolution-Independent Hotzone Hit Testing
    └── Presets evaluated against logical monitor bounds
```

### Key Technical Specifications

1. **Normalized / Relative Hotzone Representation:**
   * Rather than storing absolute desktop coordinates in `HotZone`, store positions relative to the target monitor:
     ```rust
     #[derive(Clone, Debug, Deserialize, Serialize)]
     pub struct NormalizedHotZone {
         pub monitor_index: usize,
         pub preset: HotZonePreset,     // TopLeft, BottomRight, etc.
         pub size_logical: u32,         // e.g. 120 logical px
         pub custom_offset: Option<(i32, i32)>,
     }
     ```
   * Compute actual screen hitboxes dynamically using the monitor's live `position()`, `size()`, and `scale_factor()`.

2. **Display Configuration Change Listener:**
   * Listen to Tauri's display-change events (display connection/disconnection, resolution or scale change).
   * Automatically clamp all open agent widgets into visible screen bounds to prevent windows from being "lost" off-screen when monitors are unplugged.

3. **Multi-DPI Drag Clamping:**
   * When dragging a frameless widget across monitor boundaries with differing DPIs, recalculate physical positions smoothly to prevent sudden jumping or scaling artifacts.

### Implementation Status

Completed. Hotzones are now stored as resolution-independent `NormalizedHotZone` definitions (monitor index, preset, logical size, logical offset) and resolved to physical rectangles against the live monitor list on every hit test, so they follow monitor moves, resolution changes, and DPI scaling. The hotzone editor persists the normalized definition, shows the active DPI as a percentage, and displays the resulting physical size. A display-topology monitor re-clamps all agent widgets into visible bounds and broadcasts `display-topology-changed` so the dashboard refreshes its monitor list and previews. Widget windows re-apply their logical size and re-clamp on `ScaleFactorChanged`, preventing jumps when dragged across mixed-DPI boundaries. Legacy absolute hotzones remain readable and still work.

---

## 5. Implementation Roadmap & Phasing

| Phase | Enhancement | Primary Scope | Complexity |
|---|---|---|---|
| **Phase A** | **One-Click Hook Management** | Native Rust JSON/TOML parsers, UI Integration Center with install/uninstall/verify buttons, eliminate dev-only Node script dependency. | Medium |
| **Phase B** | **Audio & Ambient Alerts** | Web Audio API / Rust chime synthesizer, volume & DND settings, `@tauri-apps/plugin-notification` integration for background toasts. | Low–Medium |
| **Phase C** | **Multi-DPI & Monitor Precision** | Monitor-relative hotzone coordinates, dynamic DPI scale conversion, display topology listener and out-of-bounds recovery. | Medium |
| **Phase D** | **Click-to-Focus Actionability** | Hook payload enrichment with process metadata, platform-specific window activation (Windows/macOS/Linux), fallback project launcher. | High |

---

## 6. Summary

Implementing these four initiatives elevates AgentLight from a status indicator into a **seamless, two-way developer control plane**:
* **Setup is effortless** via in-app configuration.
* **Alerts are unmissable** via balanced visual and auditory signals.
* **Interaction is immediate** by connecting status signals directly back to active code.
* **Display behavior is rock-solid** across all monitor and DPI topologies.
