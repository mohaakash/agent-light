import { installHooks } from "./hooks-lib.mjs";

try {
  const { hooksPath, backupPath } = installHooks();
  console.log(`AgentLight Codex hooks installed in ${hooksPath}`);
  console.log(`Original config backup: ${backupPath}`);
} catch (error) {
  console.error(`Could not install AgentLight Codex hooks: ${error.message}`);
  process.exitCode = 1;
}
