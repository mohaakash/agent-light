import { installHooks } from "./hooks-lib.mjs";

try {
  const { settingsPath, backupPath } = installHooks();
  console.log(`AgentLight Claude hooks installed in ${settingsPath}`);
  console.log(`Original config backup: ${backupPath}`);
} catch (error) {
  console.error(`Could not install AgentLight Claude hooks: ${error.message}`);
  process.exitCode = 1;
}
