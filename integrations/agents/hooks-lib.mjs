import { existsSync, mkdirSync, readFileSync, renameSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { join, resolve } from "node:path";
import { randomUUID } from "node:crypto";

const AGENT_CONFIGS = {
  cursor: {
    name: "Cursor",
    relativePath: join(".cursor", "hooks.json"),
    kind: "cursor",
    events: ["sessionStart", "sessionEnd", "beforeSubmitPrompt", "preToolUse", "postToolUse", "postToolUseFailure", "subagentStart", "subagentStop", "preCompact", "afterAgentThought", "stop"],
  },
  gemini: {
    name: "Gemini CLI",
    relativePath: join(".gemini", "settings.json"),
    kind: "nested",
    events: ["SessionStart", "SessionEnd", "BeforeAgent", "AfterAgent", "BeforeTool", "AfterTool", "Notification", "PreCompress"],
  },
  windsurf: {
    name: "Windsurf",
    relativePath: join(".codeium", "windsurf", "hooks.json"),
    kind: "flat",
    events: ["pre_read_code", "post_read_code", "pre_write_code", "post_write_code", "pre_run_command", "post_run_command", "pre_mcp_tool_use", "post_mcp_tool_use", "pre_user_prompt", "post_cascade_response", "post_cascade_response_with_transcript", "post_setup_worktree"],
  },
  copilot: {
    name: "GitHub Copilot CLI",
    relativePath: join(".copilot", "hooks", "hooks.json"),
    kind: "copilot",
    events: ["sessionStart", "sessionEnd", "userPromptSubmitted", "preToolUse", "postToolUse", "errorOccurred", "agentStop", "subagentStart", "subagentStop", "preCompact", "permissionRequest"],
  },
  antigravity: {
    name: "Antigravity CLI",
    relativePath: join(".gemini", "config", "hooks.json"),
    kind: "antigravity",
    events: ["SessionStart", "PreInvocation", "PreToolUse", "PostToolUse", "Stop"],
  },
};

export const SUPPORTED_AGENT_IDS = Object.keys(AGENT_CONFIGS);

export function getAgentConfig(agentId, homeDir = homedir()) {
  const config = AGENT_CONFIGS[agentId];
  if (!config) throw new Error(`Unsupported AgentLight adapter: ${agentId}`);
  return { ...config, filePath: join(homeDir, config.relativePath) };
}

export function getAgentHookCommand(agentId, projectRoot = process.cwd(), event = "") {
  getAgentConfig(agentId);
  const hookPath = resolve(projectRoot, "integrations/agents/hook.mjs").replaceAll("\\", "/");
  const nodePath = process.execPath.replaceAll("\\", "/");
  return `"${nodePath}" "${hookPath}" ${agentId}${event ? ` ${event}` : ""}`;
}

function readJson(filePath) {
  try {
    return JSON.parse(readFileSync(filePath, "utf8"));
  } catch (error) {
    if (error.code === "ENOENT") return {};
    throw error;
  }
}

function writeJsonAtomically(filePath, config) {
  mkdirSync(join(filePath, ".."), { recursive: true });
  const temporaryPath = `${filePath}.${randomUUID()}.tmp`;
  writeFileSync(temporaryPath, `${JSON.stringify(config, null, 2)}\n`, "utf8");
  renameSync(temporaryPath, filePath);
}

function managedText(value) {
  const text = typeof value === "string" ? value : JSON.stringify(value);
  return text.includes("integrations/agents/hook.mjs") || text.includes("agentlight");
}

function removeManagedEntries(entries) {
  if (!Array.isArray(entries)) return [];
  return entries.filter((entry) => {
    if (!entry || typeof entry !== "object") return true;
    if (Array.isArray(entry.hooks)) {
      entry.hooks = entry.hooks.filter((hook) => !managedText(hook));
      return entry.hooks.length > 0;
    }
    return !managedText(entry);
  });
}

function ensureHooksObject(config) {
  if (!config.hooks || typeof config.hooks !== "object" || Array.isArray(config.hooks)) config.hooks = {};
}

function installCursor(config, commandFor, events) {
  ensureHooksObject(config);
  for (const event of events) {
    const entries = removeManagedEntries(config.hooks[event]);
    entries.push({ command: commandFor(event) });
    config.hooks[event] = entries;
  }
}

function installNested(config, commandFor, events) {
  ensureHooksObject(config);
  for (const event of events) {
    const entries = removeManagedEntries(config.hooks[event]);
    entries.push({ matcher: "*", hooks: [{ type: "command", command: commandFor(event), name: "agentlight", timeout: 30 }] });
    config.hooks[event] = entries;
  }
}

function installFlat(config, commandFor, events) {
  ensureHooksObject(config);
  for (const event of events) {
    const entries = removeManagedEntries(config.hooks[event]);
    entries.push({ command: commandFor(event), show_output: false });
    config.hooks[event] = entries;
  }
}

function installCopilot(config, commandFor, events) {
  ensureHooksObject(config);
  for (const event of events) {
    const entries = removeManagedEntries(config.hooks[event]);
    const command = commandFor(event);
    entries.push({ type: "command", bash: command, powershell: command, timeoutSec: event === "permissionRequest" ? 600 : 5 });
    config.hooks[event] = entries;
  }
}

function installAntigravity(config, commandFor, events) {
  const agentLight = config.agentlight && typeof config.agentlight === "object" && !Array.isArray(config.agentlight)
    ? config.agentlight
    : {};
  agentLight.enabled = true;
  for (const event of events) {
    const handler = { type: "command", command: commandFor(event), timeout: 3 };
    agentLight[event] = event === "PreToolUse" || event === "PostToolUse"
      ? [{ matcher: "*", hooks: [handler] }]
      : [handler];
  }
  config.agentlight = agentLight;
}

export function installHooks({ agentId, homeDir = homedir(), projectRoot = process.cwd() } = {}) {
  const agent = getAgentConfig(agentId, homeDir);
  const commandFor = (event) => getAgentHookCommand(agentId, projectRoot, event);
  const config = readJson(agent.filePath);

  if (agent.kind === "cursor") installCursor(config, commandFor, agent.events);
  if (agent.kind === "nested") installNested(config, commandFor, agent.events);
  if (agent.kind === "flat") installFlat(config, commandFor, agent.events);
  if (agent.kind === "copilot") installCopilot(config, commandFor, agent.events);
  if (agent.kind === "antigravity") installAntigravity(config, commandFor, agent.events);

  writeJsonAtomically(agent.filePath, config);
  return { ...agent, command: commandFor(agent.events[0]) };
}

export function isHooksInstalled({ agentId, homeDir = homedir(), projectRoot = process.cwd() } = {}) {
  const agent = getAgentConfig(agentId, homeDir);
  if (!existsSync(agent.filePath)) return false;
  try {
    const config = readJson(agent.filePath);
    if (agent.kind === "antigravity") {
      const agentLight = config.agentlight;
      if (!agentLight || typeof agentLight !== "object" || Array.isArray(agentLight) || agentLight.enabled === false) return false;
      return agent.events.every((event) => {
        const expectedCommand = getAgentHookCommand(agentId, projectRoot, event);
        const entries = agentLight[event];
        if (!Array.isArray(entries)) return false;
        return (event === "PreToolUse" || event === "PostToolUse")
          ? entries.some((entry) => (entry.hooks ?? []).some((hook) => hook.command === expectedCommand))
          : entries.some((hook) => hook.command === expectedCommand);
      });
    }
    if (!config.hooks || typeof config.hooks !== "object") return false;
    return agent.events.every((event) => {
      const expectedCommand = getAgentHookCommand(agentId, projectRoot, event);
      const entries = config.hooks[event];
      if (!Array.isArray(entries)) return false;
      return entries.some((entry) => {
        if (agent.kind === "cursor" || agent.kind === "flat") return entry.command === expectedCommand;
        if (agent.kind === "copilot") return entry.bash === expectedCommand || entry.powershell === expectedCommand;
        return (entry.hooks ?? []).some((hook) => hook.command === expectedCommand);
      });
    });
  } catch {
    return false;
  }
}

export function uninstallHooks({ agentId, homeDir = homedir(), projectRoot = process.cwd() } = {}) {
  const agent = getAgentConfig(agentId, homeDir);
  if (!existsSync(agent.filePath)) return { ...agent, removed: 0 };
  const config = readJson(agent.filePath);
  if (agent.kind === "antigravity") {
    if (!config.agentlight || typeof config.agentlight !== "object" || Array.isArray(config.agentlight)) {
      return { ...agent, removed: 0 };
    }
    delete config.agentlight;
    writeJsonAtomically(agent.filePath, config);
    return { ...agent, removed: agent.events.length, command: getAgentHookCommand(agentId, projectRoot) };
  }
  let removed = 0;
  ensureHooksObject(config);
  for (const event of agent.events) {
    const before = Array.isArray(config.hooks[event]) ? config.hooks[event].length : 0;
    const entries = removeManagedEntries(config.hooks[event]);
    removed += before - entries.length;
    if (entries.length) config.hooks[event] = entries;
    else delete config.hooks[event];
  }
  if (removed) writeJsonAtomically(agent.filePath, config);
  return { ...agent, removed, command: getAgentHookCommand(agentId, projectRoot) };
}
