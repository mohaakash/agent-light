import {
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  renameSync,
  writeFileSync,
} from "node:fs";
import { homedir } from "node:os";
import { join, resolve } from "node:path";
import { randomUUID } from "node:crypto";

export const HOOK_EVENTS = [
  "SessionStart",
  "SessionEnd",
  "UserPromptSubmit",
  "PermissionRequest",
  "PreToolUse",
  "PostToolUse",
  "Stop",
  "Interrupt",
];

export function getCodexHome(env = process.env) {
  return env.CODEX_HOME || join(homedir(), ".codex");
}

export function getAgentLightHookCommand(projectRoot = process.cwd()) {
  const hookPath = resolve(projectRoot, "integrations/codex/hook.mjs").replaceAll("\\", "/");
  return `node "${hookPath}"`;
}

function readConfig(hooksPath) {
  if (!existsSync(hooksPath)) return { hooks: {} };
  const config = JSON.parse(readFileSync(hooksPath, "utf8"));
  if (!config || typeof config !== "object" || Array.isArray(config)) {
    throw new Error("Codex hooks.json must contain a JSON object");
  }
  if (config.hooks !== undefined && (!config.hooks || typeof config.hooks !== "object" || Array.isArray(config.hooks))) {
    throw new Error("Codex hooks.json 'hooks' must contain an object");
  }
  config.hooks ??= {};
  return config;
}

function writeConfigAtomically(hooksPath, config) {
  const temporaryPath = `${hooksPath}.${randomUUID()}.tmp`;
  writeFileSync(temporaryPath, `${JSON.stringify(config, null, 2)}\n`, "utf8");
  renameSync(temporaryPath, hooksPath);
}

export function installHooks({ codexHome = getCodexHome(), projectRoot = process.cwd() } = {}) {
  mkdirSync(codexHome, { recursive: true });
  const hooksPath = join(codexHome, "hooks.json");
  const configPath = join(codexHome, "config.toml");
  const backupPath = `${hooksPath}.agentlight-backup`;
  const configBackupPath = `${configPath}.agentlight-backup`;
  const config = readConfig(hooksPath);
  const hookCommand = getAgentLightHookCommand(projectRoot);

  if (existsSync(hooksPath) && !existsSync(backupPath)) {
    copyFileSync(hooksPath, backupPath);
  }

  for (const event of HOOK_EVENTS) {
    const groups = config.hooks[event] ?? [];
    if (!Array.isArray(groups)) {
      throw new Error(`Codex hooks.${event} must be an array`);
    }
    const hookDefinition = {
      type: "command",
      command: hookCommand,
      ...(process.platform === "win32" ? { commandWindows: hookCommand } : {}),
      ...(event === "SessionEnd" ? {} : { async: true }),
      timeout: 3,
    };
    const existingHook = groups.flatMap((group) => group.hooks ?? [])
      .find((hook) => hook.type === "command" && hook.command === hookCommand);
    if (existingHook) {
      Object.assign(existingHook, hookDefinition);
      if (event === "SessionEnd") delete existingHook.async;
      if (process.platform !== "win32") delete existingHook.commandWindows;
    } else {
      groups.push({
        hooks: [hookDefinition],
      });
    }
    config.hooks[event] = groups;
  }

  writeConfigAtomically(hooksPath, config);
  const featureChanged = ensureCodexHooksEnabled(configPath, configBackupPath);
  return { hooksPath, backupPath, configPath, configBackupPath, featureChanged, hookCommand };
}

export function isHooksInstalled({ codexHome = getCodexHome(), projectRoot = process.cwd() } = {}) {
  const hooksPath = join(codexHome, "hooks.json");
  const configPath = join(codexHome, "config.toml");
  if (!existsSync(hooksPath) || !existsSync(configPath)) return false;
  try {
    const config = readConfig(hooksPath);
    const hookReady = HOOK_EVENTS.every((event) => (config.hooks[event] ?? [])
      .some((group) => (group.hooks ?? []).some((hook) => hook.type === "command" && hook.command === getAgentLightHookCommand(projectRoot))));
    const features = readFileSync(configPath, "utf8");
    return hookReady && /\[features\][\s\S]*?\bhooks\s*=\s*true/.test(features);
  } catch {
    return false;
  }
}

export function ensureCodexHooksEnabled(configPath, backupPath = `${configPath}.agentlight-backup`) {
  let text = "";
  try {
    text = readFileSync(configPath, "utf8");
  } catch (error) {
    if (error.code !== "ENOENT") throw error;
  }

  if (existsSync(configPath) && !existsSync(backupPath)) copyFileSync(configPath, backupPath);
  if (!text.trim()) {
    writeFileSync(configPath, "[features]\nhooks = true\n", "utf8");
    return true;
  }

  const lines = text.split(/\r?\n/);
  const newline = text.includes("\r\n") ? "\r\n" : "\n";
  const sectionPattern = /^\s*\[[^\]]+\]\s*$/;
  const featuresAt = lines.findIndex((line) => /^\s*\[features\]\s*$/.test(line));
  if (featuresAt >= 0) {
    let sectionEnd = lines.length;
    for (let index = featuresAt + 1; index < lines.length; index += 1) {
      if (sectionPattern.test(lines[index])) {
        sectionEnd = index;
        break;
      }
    }
    for (let index = featuresAt + 1; index < sectionEnd; index += 1) {
      const match = lines[index].match(/^(\s*hooks\s*=\s*)(true|false)\s*$/);
      if (!match) continue;
      if (match[2] === "true") return false;
      lines[index] = `${match[1]}true`;
      writeFileSync(configPath, lines.join(newline), "utf8");
      return true;
    }
    lines.splice(featuresAt + 1, 0, "hooks = true");
    writeFileSync(configPath, lines.join(newline), "utf8");
    return true;
  }

  writeFileSync(configPath, `${text.replace(/\s*$/, "")}\n\n[features]\nhooks = true\n`, "utf8");
  return true;
}

export function uninstallHooks({ codexHome = getCodexHome(), projectRoot = process.cwd() } = {}) {
  const hooksPath = join(codexHome, "hooks.json");
  if (!existsSync(hooksPath)) return { hooksPath, removed: 0 };

  const config = readConfig(hooksPath);
  const hookCommand = getAgentLightHookCommand(projectRoot);
  let removed = 0;

  for (const [event, groups] of Object.entries(config.hooks ?? {})) {
    if (!Array.isArray(groups)) continue;
    const updatedGroups = groups
      .map((group) => {
        if (!Array.isArray(group.hooks)) return group;
        const remainingHooks = group.hooks.filter((hook) => {
          const isAgentLight = hook.type === "command" && hook.command === hookCommand;
          if (isAgentLight) removed += 1;
          return !isAgentLight;
        });
        return { ...group, hooks: remainingHooks };
      })
      .filter((group) => !Array.isArray(group.hooks) || group.hooks.length > 0);

    if (updatedGroups.length > 0) config.hooks[event] = updatedGroups;
    else delete config.hooks[event];
  }

  if (removed > 0) writeConfigAtomically(hooksPath, config);
  return { hooksPath, removed };
}
