# AgentLight agent adapters

AgentLight installs global hook adapters for Cursor, Gemini CLI, Windsurf, GitHub Copilot CLI, and Antigravity. The adapters preserve the agent's existing configuration, remove only AgentLight-managed entries when reinstalled, and forward normalized lifecycle metadata to the local AgentLight bridge.

Install one adapter manually from the project root:

```bash
node integrations/agents/install.mjs cursor
node integrations/agents/install.mjs gemini
node integrations/agents/install.mjs windsurf
node integrations/agents/install.mjs copilot
node integrations/agents/install.mjs antigravity
```

The dashboard's Agents tab runs the same installers globally. Replace the final argument with `cursor`, `gemini`, `windsurf`, `copilot`, or `antigravity`.

To remove an adapter, run:

```bash
node integrations/agents/uninstall.mjs <agent>
```

Hooks are local-only and fail open if AgentLight is not running, so they do not block the agent. The shared adapter has automated installer and mapping tests; live smoke testing with each installed agent remains outstanding.
