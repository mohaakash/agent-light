# AgentLight Claude Code integration

This adapter maps Claude Code lifecycle hooks to AgentLight's authenticated loopback API. It sends status metadata only and does not return permission decisions.

## Requirements

- AgentLight running locally at least once to create its local API bridge configuration.
- Claude Code with command hooks enabled.
- Node.js available to hook commands.

## Install

Run from the AgentLight project root:

```powershell
node integrations/claude/install.mjs
```

The installer merges handlers into `~/.claude/settings.json`, creates one first-install backup at `settings.json.agentlight-backup`, and uses an absolute path to this checkout. It is idempotent and preserves other settings and hooks.

## Uninstall

```powershell
node integrations/claude/uninstall.mjs
```

Uninstall removes only handlers matching this checkout's `hook.mjs` command. The original backup is kept.

## Event mapping

| Claude Code hook | AgentLight event | Display |
|---|---|---|
| `SessionStart` | `session_started` | Idle |
| `UserPromptSubmit`, `PreToolUse`, `PostToolUse` | `agent_working` | Working |
| `PermissionRequest` | `permission_required` | Waiting |
| `Stop` | `task_completed` | Completed for five seconds |
| `StopFailure` | `session_error` | Error |
| `SessionEnd` | `session_ended` | Idle |

## Privacy and behavior

Only the session ID, working-directory basename, lifecycle event, generated event ID, and timestamp are sent. Prompts, transcript paths, tool names, tool arguments, errors, and assistant text are not sent. Hook commands run asynchronously and silently; failure to reach AgentLight does not block Claude Code. Permission requests are observed only and never approved by AgentLight.

## References

- [Claude Code hooks reference](https://code.claude.com/docs/en/hooks)
- [AgentLight local API](../../docs/LOCAL_API.md)
