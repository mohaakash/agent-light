import { copyFileSync, existsSync, mkdirSync, readFileSync, renameSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { join, resolve } from "node:path";
import { randomUUID } from "node:crypto";

export const HOOK_EVENTS = [
  "SessionStart", "SessionEnd", "UserPromptSubmit", "PermissionRequest",
  "PreToolUse", "PostToolUse", "Stop", "StopFailure",
];

export function getClaudeSettingsDir(env = process.env) {
  return env.CLAUDE_CONFIG_DIR || join(homedir(), ".claude");
}

export function getAgentLightHookCommand(projectRoot = process.cwd()) {
  const hookPath = resolve(projectRoot, "integrations/claude/hook.mjs").replaceAll("\\", "/");
  return `node "${hookPath}"`;
}

function readConfig(path) {
  if (!existsSync(path)) return { hooks: {} };
  const config = JSON.parse(readFileSync(path, "utf8"));
  if (!config || typeof config !== "object" || Array.isArray(config)) throw new Error("Claude settings.json must contain an object");
  if (config.hooks !== undefined && (!config.hooks || typeof config.hooks !== "object" || Array.isArray(config.hooks))) {
    throw new Error("Claude settings.json 'hooks' must contain an object");
  }
  config.hooks ??= {};
  return config;
}

function writeConfig(path, config) {
  const temporary = `${path}.${randomUUID()}.tmp`;
  writeFileSync(temporary, `${JSON.stringify(config, null, 2)}\n`, "utf8");
  renameSync(temporary, path);
}

export function installHooks({ claudeConfigDir = getClaudeSettingsDir(), projectRoot = process.cwd() } = {}) {
  mkdirSync(claudeConfigDir, { recursive: true });
  const settingsPath = join(claudeConfigDir, "settings.json");
  const backupPath = `${settingsPath}.agentlight-backup`;
  const config = readConfig(settingsPath);
  const command = getAgentLightHookCommand(projectRoot);
  if (existsSync(settingsPath) && !existsSync(backupPath)) copyFileSync(settingsPath, backupPath);

  for (const event of HOOK_EVENTS) {
    const groups = config.hooks[event] ?? [];
    if (!Array.isArray(groups)) throw new Error(`Claude hooks.${event} must be an array`);
    const installed = groups.some((group) => group.hooks?.some((hook) => hook.type === "command" && hook.command === command));
    if (!installed) groups.push({ hooks: [{ type: "command", command, async: true, timeout: 3 }] });
    config.hooks[event] = groups;
  }
  writeConfig(settingsPath, config);
  return { settingsPath, backupPath, command };
}

export function uninstallHooks({ claudeConfigDir = getClaudeSettingsDir(), projectRoot = process.cwd() } = {}) {
  const settingsPath = join(claudeConfigDir, "settings.json");
  if (!existsSync(settingsPath)) return { settingsPath, removed: 0 };
  const config = readConfig(settingsPath);
  const command = getAgentLightHookCommand(projectRoot);
  let removed = 0;
  for (const [event, groups] of Object.entries(config.hooks)) {
    if (!Array.isArray(groups)) continue;
    const updated = groups.map((group) => {
      if (!Array.isArray(group.hooks)) return group;
      const hooks = group.hooks.filter((hook) => {
        if (hook.type === "command" && hook.command === command) { removed += 1; return false; }
        return true;
      });
      return { ...group, hooks };
    }).filter((group) => !Array.isArray(group.hooks) || group.hooks.length > 0);
    if (updated.length) config.hooks[event] = updated;
    else delete config.hooks[event];
  }
  if (removed) writeConfig(settingsPath, config);
  return { settingsPath, removed };
}
