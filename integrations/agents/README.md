# AgentLight agent adapters

AgentLight installs global hook adapters for Cursor, Gemini CLI, Windsurf, and GitHub Copilot CLI. The adapters preserve the agent's existing configuration, remove only AgentLight-managed entries when reinstalled, and forward normalized lifecycle metadata to the local AgentLight bridge.

Install one adapter manually from the project root:

```bash
node integrations/agents/install.mjs cursor
node integrations/agents/install.mjs gemini
node integrations/agents/install.mjs windsurf
node integrations/agents/install.mjs copilot
```

The dashboard's Agents tab runs the same installers globally. Hooks are local-only and fail open if AgentLight is not running, so they do not block the agent.
