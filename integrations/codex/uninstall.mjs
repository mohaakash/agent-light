import { uninstallHooks } from "./hooks-lib.mjs";

try {
  const { hooksPath, removed } = uninstallHooks();
  console.log(`Removed ${removed} AgentLight Codex hook handler(s) from ${hooksPath}`);
  console.log("Other hooks were preserved. The first-install backup was left in place.");
} catch (error) {
  console.error(`Could not uninstall AgentLight Codex hooks: ${error.message}`);
  process.exitCode = 1;
}
