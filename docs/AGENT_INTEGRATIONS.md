# Agent Integrations

## 1. Strategy

Each coding agent has its own event model and extension mechanism. AgentLight must use documented, supported mechanisms and normalize their events into the canonical schema in `STATUS_MODEL.md`.

Do not assume that all agents expose identical events or that their APIs remain stable.

## 2. Integration status

Implemented adapters:

- OpenAI Codex
- Claude Code
- Cursor
- Gemini CLI
- Windsurf
- GitHub Copilot CLI
- Google Antigravity

Planned adapters:

- OpenCode
- Pi coding agent

The implemented adapters use documented lifecycle hooks where available and send normalized status metadata to the authenticated local API. Support means that AgentLight can install and remove the adapter and process its mapped events; live smoke testing against each installed agent remains outstanding.

Before implementing an adapter:
1. Consult the current official documentation for the exact agent and version.
2. Identify supported hooks, plugins, extensions or event APIs.
3. Identify permission/approval, start, progress, completion, error and end events.
4. Record payload examples and lifecycle limitations.
5. Implement a minimal adapter.
6. Test against the installed agent version.
7. Document setup, uninstall and troubleshooting.

## 3. Adapter contract

An adapter should:
- Have a stable identifier, e.g. `codex`, `claude`, `opencode`, `pi`, `antigravity`.
- Map source events to canonical AgentLight events.
- Provide a stable session ID where possible.
- Send only status metadata.
- Handle AgentLight not running.
- Avoid blocking or disrupting the coding agent.
- Never auto-approve permissions.
- Be idempotent where practical.
- Report its version and installation state.

## 4. Suggested mapping template

Complete this table separately for each agent using current official documentation.

| Source lifecycle condition | Canonical event | Notes |
|---|---|---|
| Session starts | `session_started` | Determine stable session ID |
| Agent begins/continues work | `agent_working` | May be inferred from tool lifecycle |
| Human permission required | `permission_required` | Must not imply approval |
| Human input required | `user_input_required` | Map only when reliably detectable |
| Task finishes successfully | `task_completed` | Define exact completion semantics |
| Session ends | `session_ended` | Clear or finalize session |
| Error occurs | `session_error` | Avoid sending raw error output |
| Periodic liveness | `heartbeat` | Only if supported |

## 5. Installation model

Preferred UX:
- User selects an agent integration in AgentLight.
- App checks whether the agent is installed and identifies relevant configuration location.
- App explains the changes it intends to make.
- User explicitly approves installation.
- App backs up or safely merges existing configuration.
- App provides an uninstall/restore path.
- App verifies installation where possible.

Never blindly overwrite an agent's existing settings or hooks.

## 6. Integration documentation template

Every adapter should have a README containing:
- Supported agent and tested version.
- Required permissions.
- Official documentation links.
- Installation steps.
- Configuration changes.
- Event mapping table.
- Known limitations.
- Uninstall steps.
- Troubleshooting.
- Automated/manual test procedure.

## 7. Implementation sequence

Codex was implemented end-to-end first to validate the event pipeline. Claude Code followed with the same bridge contract. Cursor, Gemini CLI, Windsurf, GitHub Copilot CLI, and Antigravity now use the shared adapter installer and hook bridge. New adapters should still be added one at a time after their lifecycle interfaces and installed versions are verified.

## 8. Codex integration — initial implementation

AgentLight's first adapter uses Codex lifecycle hooks and the authenticated loopback endpoint described in `LOCAL_API.md`. The adapter is intentionally metadata-only and failure-tolerant.

### Event mapping

| Codex hook | Canonical event | Display state |
|---|---|---|
| `SessionStart` | `session_started` | `IDLE` |
| `UserPromptSubmit` | `agent_working` | `WORKING` |
| `PreToolUse`, `PostToolUse` | `agent_working` | `WORKING` |
| `PermissionRequest` | `permission_required` | `WAITING` |
| `Stop` | `task_completed` | `COMPLETED` for five seconds, then `IDLE` |
| `Interrupt` | `session_idle` | `IDLE` |
| `SessionEnd` | `session_ended` | `IDLE` |

The adapter is implemented in `integrations/codex/hook.mjs`. The example hook configuration is in `integrations/codex/hooks.json`. For a user-level installation that preserves existing hooks, run `node integrations/codex/install.mjs` from the project root; it creates one backup at `~/.codex/hooks.json.agentlight-backup` before merging the AgentLight handlers. Alternatively, merge the entries manually or install the same command hooks through Codex's `/hooks` UI. Start AgentLight before using Codex so it can create the bridge configuration under its app config directory.

The hook exits successfully when AgentLight is stopped, unavailable or not configured. This prevents the integration from blocking or changing Codex behavior.

## 9. Claude Code integration — initial implementation

The second adapter uses Claude Code's documented command hooks and the same authenticated local event API. See [`integrations/claude/README.md`](../integrations/claude/README.md) for setup, uninstall, privacy, and lifecycle mapping. Claude's `StopFailure` hook can report an error signal without transmitting error details.

The hook installation and live event delivery have not yet been smoke-tested inside a running Claude Code session.

## 10. Shared adapters

Cursor, Gemini CLI, Windsurf, GitHub Copilot CLI, and Antigravity use the shared implementation in `integrations/agents/`. Install one from the repository root with:

```sh
node integrations/agents/install.mjs cursor
node integrations/agents/install.mjs gemini
node integrations/agents/install.mjs windsurf
node integrations/agents/install.mjs copilot
node integrations/agents/install.mjs antigravity
```

The shared installer preserves existing configuration, creates or updates only AgentLight-managed entries, and provides matching uninstall behavior. Agent-specific event maps are defined in `integrations/agents/hook.mjs`. The adapters fail open when AgentLight is unavailable and do not send prompts, source code, tool arguments, or terminal output.
