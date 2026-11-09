import { randomUUID } from "node:crypto";
import { basename, join } from "node:path";
import { homedir, platform } from "node:os";
import { existsSync, readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

const EVENT_MAP = {
  SessionStart: "session_started",
  SessionEnd: "session_ended",
  UserPromptSubmit: "agent_working",
  PermissionRequest: "permission_required",
  PreToolUse: "agent_working",
  PostToolUse: "agent_working",
  Stop: "task_completed",
  Interrupt: "session_idle",
};

export function configDirectory() {
  if (process.env.AGENTLIGHT_CONFIG_DIR) return process.env.AGENTLIGHT_CONFIG_DIR;
  if (platform() === "win32") {
    return join(process.env.APPDATA ?? join(homedir(), "AppData", "Roaming"), "com.agentlight.app");
  }
  if (platform() === "darwin") {
    return join(homedir(), "Library", "Application Support", "com.agentlight.app");
  }
  return join(process.env.XDG_CONFIG_HOME ?? join(homedir(), ".config"), "com.agentlight.app");
}

export function normalizeHookInput(input) {
  const event = EVENT_MAP[input?.hook_event_name];
  if (!event || typeof input.session_id !== "string" || input.session_id.length === 0) return null;
  return {
    schema_version: 1,
    event_id: randomUUID(),
    agent: "codex",
    session_id: input.session_id,
    project: typeof input.cwd === "string" ? basename(input.cwd) : null,
    event,
    occurred_at: new Date().toISOString(),
  };
}

export async function forwardHookInput(input, { bridgePath = join(configDirectory(), "codex-bridge.json"), fetchImpl = fetch } = {}) {
  const payload = normalizeHookInput(input);
  if (!payload || !existsSync(bridgePath)) return false;

  try {
    const bridge = JSON.parse(readFileSync(bridgePath, "utf8"));
    if (typeof bridge.endpoint !== "string" || typeof bridge.token !== "string") return false;
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 1000);
    try {
      const response = await fetchImpl(bridge.endpoint, {
        method: "POST",
        headers: {
          Authorization: `Bearer ${bridge.token}`,
          "Content-Type": "application/json",
        },
        body: JSON.stringify(payload),
        signal: controller.signal,
      });
      return response.ok;
    } finally {
      clearTimeout(timeout);
    }
  } catch {
    // Integration outages must never stop Codex from working.
    return false;
  }
}

async function readStdin() {
  const chunks = [];
  for await (const chunk of process.stdin) chunks.push(chunk);
  return Buffer.concat(chunks).toString("utf8");
}

async function main() {
  await forwardHookInput(JSON.parse(await readStdin()));
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch(() => undefined);
}
