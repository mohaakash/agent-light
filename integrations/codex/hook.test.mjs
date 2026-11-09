import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { createServer } from "node:http";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";

import { forwardHookInput, normalizeHookInput } from "./hook.mjs";

test("Codex hook process sends only normalized status metadata over HTTP", async () => {
  let requestPayload;
  let requestAuthorization;
  const server = createServer((request, response) => {
    requestAuthorization = request.headers.authorization;
    const chunks = [];
    request.on("data", (chunk) => chunks.push(chunk));
    request.on("end", () => {
      requestPayload = JSON.parse(Buffer.concat(chunks).toString("utf8"));
      response.writeHead(200, { "content-type": "application/json" });
      response.end(JSON.stringify({ accepted: true }));
    });
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  const address = server.address();
  const configDirectory = mkdtempSync(join(tmpdir(), "agentlight-hook-").replaceAll("\\", "/"));
  const bridgePath = join(configDirectory, "codex-bridge.json");
  writeFileSync(bridgePath, JSON.stringify({ endpoint: `http://127.0.0.1:${address.port}/api/v1/events`, token: "test-token" }));

  try {
    const child = spawn(process.execPath, [fileURLToPath(new URL("./hook.mjs", import.meta.url))], {
      env: { ...process.env, AGENTLIGHT_CONFIG_DIR: configDirectory },
      stdio: ["pipe", "ignore", "ignore"],
    });
    const exited = new Promise((resolve, reject) => {
      child.once("error", reject);
      child.once("close", (code) => resolve(code));
    });
    child.stdin.end(JSON.stringify({
      hook_event_name: "PermissionRequest",
      session_id: "session-123",
      cwd: "C:/work/private-project",
      prompt: "must not be forwarded",
      tool_input: { secret: "must not be forwarded" },
      transcript_path: "C:/private/transcript.jsonl",
    }));

    assert.equal(await exited, 0);
    assert.equal(requestAuthorization, "Bearer test-token");
    assert.equal(requestPayload.agent, "codex");
    assert.equal(requestPayload.session_id, "session-123");
    assert.equal(requestPayload.project, "private-project");
    assert.equal(requestPayload.event, "permission_required");
    assert.equal(requestPayload.schema_version, 1);
    assert.ok(requestPayload.occurred_at);
    assert.equal("prompt" in requestPayload, false);
    assert.equal("tool_input" in requestPayload, false);
    assert.equal("transcript_path" in requestPayload, false);
  } finally {
    await new Promise((resolve, reject) => server.close((error) => error ? reject(error) : resolve()));
    rmSync(configDirectory, { recursive: true, force: true });
  }
});

test("hook mapping handles supported lifecycle events and ignores unknown ones", () => {
  const session = { session_id: "s1" };
  assert.equal(normalizeHookInput({ ...session, hook_event_name: "SessionStart" }).event, "session_started");
  assert.equal(normalizeHookInput({ ...session, hook_event_name: "UserPromptSubmit" }).event, "agent_working");
  assert.equal(normalizeHookInput({ ...session, hook_event_name: "Stop" }).event, "task_completed");
  assert.equal(normalizeHookInput({ ...session, hook_event_name: "SessionEnd" }).event, "session_ended");
  assert.equal(normalizeHookInput({ ...session, hook_event_name: "Unsupported" }), null);
  assert.equal(normalizeHookInput({ hook_event_name: "Stop" }), null);
});

test("missing bridge and failed local API never fail the hook", async () => {
  assert.equal(await forwardHookInput({ hook_event_name: "Stop", session_id: "s1" }, { bridgePath: join(tmpdir(), "missing-agentlight-bridge.json") }), false);
  assert.equal(await forwardHookInput({ hook_event_name: "Stop", session_id: "s1" }, {
    bridgePath: join(tmpdir(), "missing-agentlight-bridge.json"),
    fetchImpl: async () => { throw new Error("offline"); },
  }), false);
});
