import { uninstallHooks } from "./hooks-lib.mjs";

try {
  const { settingsPath, removed } = uninstallHooks();
  console.log(`Removed ${removed} AgentLight Claude hook handler(s) from ${settingsPath}`);
  console.log("Other hooks were preserved. The first-install backup was left in place.");
} catch (error) {
  console.error(`Could not uninstall AgentLight Claude hooks: ${error.message}`);
  process.exitCode = 1;
}
