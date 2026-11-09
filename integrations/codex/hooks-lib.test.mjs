import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import { HOOK_EVENTS, installHooks, uninstallHooks, getAgentLightHookCommand } from "./hooks-lib.mjs";

function withTempCodexHome(run) {
  const codexHome = mkdtempSync(join(tmpdir(), "agentlight-codex-").replaceAll("\\", "/"));
  try {
    return run(codexHome);
  } finally {
    rmSync(codexHome, { recursive: true, force: true });
  }
}

test("install merges hooks, backs up original config, and is idempotent", () => {
  withTempCodexHome((codexHome) => {
    const hooksPath = join(codexHome, "hooks.json");
    const original = {
      description: "User hooks",
      hooks: {
        PreToolUse: [{ matcher: "Bash", hooks: [{ type: "command", command: "keep-me" }] }],
      },
    };
    writeFileSync(hooksPath, `${JSON.stringify(original, null, 2)}\n`);

    const installed = installHooks({ codexHome, projectRoot: process.cwd() });
    const merged = JSON.parse(readFileSync(hooksPath, "utf8"));
    assert.deepEqual(JSON.parse(readFileSync(installed.backupPath, "utf8")), original);
    assert.equal(merged.description, "User hooks");
    assert.equal(merged.hooks.PreToolUse[0].hooks[0].command, "keep-me");
    for (const event of HOOK_EVENTS) {
      const handlers = merged.hooks[event].flatMap((group) => group.hooks ?? []);
      const agentLightHooks = handlers.filter((hook) => hook.command === installed.hookCommand);
      assert.equal(agentLightHooks.length, 1);
      if (process.platform === "win32") assert.equal(agentLightHooks[0].commandWindows, installed.hookCommand);
      if (event === "SessionEnd") assert.equal("async" in agentLightHooks[0], false);
      else assert.equal(agentLightHooks[0].async, true);
    }

    installHooks({ codexHome, projectRoot: process.cwd() });
    const reinstalled = JSON.parse(readFileSync(hooksPath, "utf8"));
    for (const event of HOOK_EVENTS) {
      assert.equal(
        reinstalled.hooks[event].flatMap((group) => group.hooks ?? [])
          .filter((hook) => hook.command === installed.hookCommand).length,
        1,
      );
    }
  });
});

test("uninstall removes only AgentLight handlers and preserves user hooks", () => {
  withTempCodexHome((codexHome) => {
    const hooksPath = join(codexHome, "hooks.json");
    writeFileSync(hooksPath, JSON.stringify({ hooks: { PreToolUse: [{ hooks: [{ type: "command", command: "user-hook" }] }] } }));
    const installed = installHooks({ codexHome, projectRoot: process.cwd() });

    const result = uninstallHooks({ codexHome, projectRoot: process.cwd() });
    const config = JSON.parse(readFileSync(hooksPath, "utf8"));
    assert.equal(result.removed, HOOK_EVENTS.length);
    assert.deepEqual(config.hooks, { PreToolUse: [{ hooks: [{ type: "command", command: "user-hook" }] }] });
    assert.equal(readFileSync(installed.backupPath, "utf8").length > 0, true);
  });
});

test("uninstall is safe when AgentLight was never installed", () => {
  withTempCodexHome((codexHome) => {
    const hooksPath = join(codexHome, "hooks.json");
    writeFileSync(hooksPath, JSON.stringify({ hooks: { Stop: [{ hooks: [{ type: "command", command: "other" }] }] } }));
    assert.equal(uninstallHooks({ codexHome, projectRoot: process.cwd() }).removed, 0);
    assert.equal(JSON.parse(readFileSync(hooksPath, "utf8")).hooks.Stop[0].hooks[0].command, "other");
  });
});

test("install rejects malformed config instead of overwriting it", () => {
  withTempCodexHome((codexHome) => {
    const hooksPath = join(codexHome, "hooks.json");
    writeFileSync(hooksPath, "[]");
    assert.throws(() => installHooks({ codexHome, projectRoot: process.cwd() }), /must contain a JSON object/);
    assert.equal(readFileSync(hooksPath, "utf8"), "[]");
  });
});

test("installed command points to the absolute hook script path", () => {
  const command = getAgentLightHookCommand(process.cwd());
  assert.match(command, /^node ".*integrations\/codex\/hook\.mjs"$/);
});
