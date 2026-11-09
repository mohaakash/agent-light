# AgentLight Codex integration

This adapter uses Codex lifecycle hooks to update the AgentLight virtual traffic light.

## Requirements

- AgentLight running locally.
- Codex with lifecycle hooks enabled.
- Node.js available to Codex hook commands.

## Install

From the AgentLight project root:

```powershell
node integrations/codex/install.mjs
```

The installer merges handlers into `~/.codex/hooks.json` (or `$CODEX_HOME/hooks.json`) and uses an absolute path to this checkout so Codex can start the hook from any project directory. On Windows it sets Codex's `commandWindows` override. Before its first edit, it saves the original as `hooks.json.agentlight-backup`. The install is idempotent and preserves other hook groups. Codex requires new or changed hooks to be reviewed and trusted; open `/hooks` and trust the AgentLight entries. AgentLight must be running once to create the bridge configuration.

## Uninstall

From the same AgentLight checkout used during installation:

```powershell
node integrations/codex/uninstall.mjs
```

Uninstall removes only handlers whose command points to this checkout's `hook.mjs`; other hooks and the original backup remain. If you move or remove the checkout first, run uninstall from the original path or remove its exact handler command manually.

The hook command reads the bridge endpoint and bearer token from AgentLight's app configuration. If AgentLight is not running, the hook exits successfully and Codex continues normally. Turn hooks run asynchronously; `SessionEnd` runs synchronously because Codex handles that lifecycle event synchronously.

## Event mapping

| Codex hook | AgentLight state |
|---|---|
| `SessionStart` | `IDLE` |
| `UserPromptSubmit`, `PreToolUse`, `PostToolUse` | `WORKING` |
| `PermissionRequest` | `WAITING` |
| `Stop` | `COMPLETED`, then `IDLE` after five seconds |
| `Interrupt`, `SessionEnd` | `IDLE` |

## Privacy and limitations

The hook sends only the Codex session ID, project basename, canonical event name, event ID and timestamp. It does not send prompts, source code, tool arguments, transcript paths or terminal output. Session aggregation is handled by the Rust core. Codex does not provide an explicit session-error lifecycle hook in the signals currently mapped, so the adapter does not infer an error from message text or tool output. A later working event clears a permission wait; session end returns that session to idle.

Run the isolated hook and installer tests with `npm run test:codex`. These exercise the hook process against a local HTTP test server and do not edit the user's Codex configuration. They do not replace a manual smoke test in an installed Codex session.

Official references:

- [Codex lifecycle hooks](https://learn.chatgpt.com/docs/hooks)
- [AgentLight local API](../../docs/LOCAL_API.md)
