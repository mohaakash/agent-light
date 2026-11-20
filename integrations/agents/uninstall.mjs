import { uninstallHooks, SUPPORTED_AGENT_IDS } from "./hooks-lib.mjs";

const agentId = process.argv[2];
if (!SUPPORTED_AGENT_IDS.includes(agentId)) {
  console.error(`Usage: node integrations/agents/uninstall.mjs <${SUPPORTED_AGENT_IDS.join("|")}>`);
  process.exitCode = 1;
} else {
  try {
    const uninstalled = uninstallHooks({ agentId });
    console.log(`Removed ${uninstalled.removed} AgentLight ${uninstalled.name} hook handler(s) from ${uninstalled.filePath}`);
    console.log("Other hooks were preserved. The first-install backup was left in place.");
  } catch (error) {
    console.error(`Could not uninstall AgentLight ${agentId} hooks: ${error.message}`);
    process.exitCode = 1;
  }
}
