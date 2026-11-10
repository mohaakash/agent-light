import { installHooks } from "./hooks-lib.mjs";

try {
  const { hooksPath, backupPath, configPath, featureChanged } = installHooks();
  console.log(`AgentLight Codex hooks installed in ${hooksPath}`);
  console.log(`Original config backup: ${backupPath}`);
  console.log(`Codex hooks feature ${featureChanged ? "enabled" : "already enabled"} in ${configPath}`);
} catch (error) {
  console.error(`Could not install AgentLight Codex hooks: ${error.message}`);
  process.exitCode = 1;
}
