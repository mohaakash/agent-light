import { randomUUID } from "node:crypto";
import { existsSync, readFileSync } from "node:fs";
import { basename, join } from "node:path";
import { homedir, platform } from "node:os";
import { pathToFileURL } from "node:url";

const EVENT_MAP = {
  cursor: {
    sessionStart: "session_started", sessionEnd: "session_ended", beforeSubmitPrompt: "agent_working", preToolUse: "agent_working", postToolUse: "agent_working", postToolUseFailure: "session_error", subagentStart: "agent_working", subagentStop: "agent_working", preCompact: "agent_working", afterAgentThought: "agent_working", stop: "task_completed",
  },
  gemini: {
    SessionStart: "session_started", SessionEnd: "session_ended", BeforeAgent: "agent_working", AfterAgent: "task_completed", BeforeTool: "agent_working", AfterTool: "agent_working", Notification: "permission_required", PreCompress: "agent_working",
  },
  windsurf: {
    pre_read_code: "agent_working", post_read_code: "agent_working", pre_write_code: "agent_working", post_write_code: "agent_working", pre_run_command: "agent_working", post_run_command: "agent_working", pre_mcp_tool_use: "agent_working", post_mcp_tool_use: "agent_working", pre_user_prompt: "agent_working", post_cascade_response: "task_completed", post_cascade_response_with_transcript: "task_completed", post_setup_worktree: "agent_working",
  },
  copilot: {
    sessionStart: "session_started", sessionEnd: "session_ended", userPromptSubmitted: "agent_working", preToolUse: "agent_working", postToolUse: "agent_working", errorOccurred: "session_error", agentStop: "task_completed", subagentStart: "agent_working", subagentStop: "agent_working", preCompact: "agent_working", permissionRequest: "permission_required",
  },
  antigravity: {
    SessionStart: "session_started", PreInvocation: "agent_working", PreToolUse: "agent_working", PostToolUse: "agent_working", Stop: "task_completed",
  },
};

function configDirectory() {
  if (process.env.AGENTLIGHT_CONFIG_DIR) return process.env.AGENTLIGHT_CONFIG_DIR;
  if (platform() === "win32") return join(process.env.APPDATA ?? join(homedir(), "AppData", "Roaming"), "com.agentlight.app");
  if (platform() === "darwin") return join(homedir(), "Library", "Application Support", "com.agentlight.app");
  return join(process.env.XDG_CONFIG_HOME ?? join(homedir(), ".config"), "com.agentlight.app");
}

function readInput() {
  return new Promise((resolve) => {
    const chunks = [];
    process.stdin.on("data", (chunk) => chunks.push(chunk));
    process.stdin.on("end", () => resolve(Buffer.concat(chunks).toString("utf8")));
    setTimeout(() => resolve(Buffer.concat(chunks).toString("utf8")), 500).unref();
  });
}

function firstString(...values) {
  return values.find((value) => typeof value === "string" && value.trim())?.trim() ?? "";
}

function bridgePath() {
  return join(configDirectory(), "codex-bridge.json");
}

function eventForHook(agentId, eventName, input) {
  if (agentId === "antigravity" && eventName === "PreToolUse") {
    const toolName = input.toolCall?.name;
    if (toolName === "ask_permission" || toolName === "ask_question") return "permission_required";
  }
  return EVENT_MAP[agentId]?.[eventName];
}

function normalizeHookInput(agentId, input, fallbackEvent) {
  const eventName = firstString(input.hook_event_name, input.event, input.event_name, fallbackEvent);
  const event = eventForHook(agentId, eventName, input);
  if (!event) return null;
  const sessionId = firstString(input.session_id, input.sessionId, input.conversation_id, input.conversationId, input.execution_id, "default");
  const cwd = firstString(input.cwd, input.workspace_root, input.project_dir, input.directory, input.workspace_path, Array.isArray(input.workspacePaths) ? input.workspacePaths[0] : "");
  return {
    schema_version: 1,
    event_id: randomUUID(),
    agent: agentId,
    session_id: sessionId,
    project: cwd ? basename(cwd) : null,
    event,
    occurred_at: new Date().toISOString(),
  };
}

async function forwardHookInput(payload, fetchImpl = fetch) {
  const path = bridgePath();
  if (!existsSync(path)) return false;
  try {
    const bridge = JSON.parse(readFileSync(path, "utf8"));
    if (typeof bridge.endpoint !== "string" || typeof bridge.token !== "string") return false;
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 1000);
    try {
      const response = await fetchImpl(bridge.endpoint, {
        method: "POST",
        headers: { Authorization: `Bearer ${bridge.token}`, "Content-Type": "application/json" },
        body: JSON.stringify(payload),
        signal: controller.signal,
      });
      return response.ok;
    } finally {
      clearTimeout(timeout);
    }
  } catch {
    return false;
  }
}

export function normalizeHookInputForTest(agentId, input, fallbackEvent) {
  return normalizeHookInput(agentId, input, fallbackEvent);
}

async function main() {
  const agentId = process.argv[2] || "";
  const fallbackEvent = process.argv[3] || "";
  let input = {};
  try {
    const raw = await readInput();
    input = raw.trim() ? JSON.parse(raw) : {};
  } catch {
    input = {};
  }
  const payload = normalizeHookInput(agentId, input, fallbackEvent);
  process.stdout.write((agentId === "gemini" && fallbackEvent === "BeforeTool") || (agentId === "antigravity" && fallbackEvent === "PreToolUse") ? '{"decision":"allow"}\n' : "{}\n");
  if (payload) await forwardHookInput(payload);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch(() => undefined);
}
