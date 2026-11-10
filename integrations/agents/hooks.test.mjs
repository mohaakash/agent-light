import assert from "node:assert/strict";
import test from "node:test";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { getAgentHookCommand, installHooks, isHooksInstalled, uninstallHooks } from "./hooks-lib.mjs";
import { normalizeHookInputForTest } from "./hook.mjs";

test("normalizes Cursor and Gemini events into AgentLight events", () => {
  const cursor = normalizeHookInputForTest("cursor", { session_id: "cursor-1", cwd: "/tmp/demo" }, "preToolUse");
  assert.equal(cursor.agent, "cursor");
  assert.equal(cursor.event, "agent_working");
  assert.equal(cursor.project, "demo");

  const gemini = normalizeHookInputForTest("gemini", { session_id: "gemini-1" }, "AfterAgent");
  assert.equal(gemini.event, "task_completed");

  const antigravity = normalizeHookInputForTest("antigravity", { conversationId: "anti-1", workspacePaths: ["/tmp/demo"] }, "PreInvocation");
  assert.equal(antigravity.agent, "antigravity");
  assert.equal(antigravity.event, "agent_working");
  assert.equal(antigravity.project, "demo");
});

test("maps Antigravity permission and question tools to attention", () => {
  for (const toolName of ["ask_permission", "ask_question"]) {
    const event = normalizeHookInputForTest("antigravity", { conversationId: "anti-1", toolCall: { name: toolName } }, "PreToolUse");
    assert.equal(event.event, "permission_required");
  }
});

test("normalizes unknown events to null", () => {
  assert.equal(normalizeHookInputForTest("cursor", { session_id: "cursor-1" }, "unknown"), null);
});

test("uses an absolute Node executable so desktop-launched hooks do not depend on PATH", () => {
  assert.match(getAgentHookCommand("antigravity", process.cwd(), "PreInvocation"), /^"\/.+node" "/);
});

test("installs Antigravity's named global hook configuration", () => {
  const root = mkdtempSync(join(tmpdir(), "agentlight-agents-"));
  const home = mkdtempSync(join(tmpdir(), "agentlight-home-"));
  try {
    const installed = installHooks({ agentId: "antigravity", homeDir: home, projectRoot: root });
    const config = JSON.parse(readFileSync(installed.filePath, "utf8"));
    assert.equal(config.agentlight.enabled, true);
    assert.equal(config.agentlight.PreInvocation[0].command, getAgentHookCommand("antigravity", root, "PreInvocation"));
    assert.equal(config.agentlight.PreToolUse[0].hooks[0].command, getAgentHookCommand("antigravity", root, "PreToolUse"));
    assert.equal(isHooksInstalled({ agentId: "antigravity", homeDir: home, projectRoot: root }), true);
    assert.equal(uninstallHooks({ agentId: "antigravity", homeDir: home, projectRoot: root }).removed, 5);
  } finally {
    rmSync(root, { recursive: true, force: true });
    rmSync(home, { recursive: true, force: true });
  }
});

test("installs and uninstalls a global-style Cursor config idempotently", () => {
  const root = mkdtempSync(join(tmpdir(), "agentlight-agents-"));
  const home = mkdtempSync(join(tmpdir(), "agentlight-home-"));
  try {
    assert.equal(isHooksInstalled({ agentId: "cursor", homeDir: home, projectRoot: root }), false);
    const installed = installHooks({ agentId: "cursor", homeDir: home, projectRoot: root });
    const config = JSON.parse(readFileSync(installed.filePath, "utf8"));
    assert.equal(config.hooks.sessionStart.length, 1);
    assert.equal(config.hooks.sessionStart[0].command, getAgentHookCommand("cursor", root, "sessionStart"));
    assert.equal(isHooksInstalled({ agentId: "cursor", homeDir: home, projectRoot: root }), true);
    installHooks({ agentId: "cursor", homeDir: home, projectRoot: root });
    const repeated = JSON.parse(readFileSync(installed.filePath, "utf8"));
    assert.equal(repeated.hooks.sessionStart.length, 1);
    assert.equal(uninstallHooks({ agentId: "cursor", homeDir: home, projectRoot: root }).removed, 11);
  } finally {
    rmSync(root, { recursive: true, force: true });
    rmSync(home, { recursive: true, force: true });
  }
});
