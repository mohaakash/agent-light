import { installHooks, SUPPORTED_AGENT_IDS } from "./hooks-lib.mjs";

const agentId = process.argv[2];
if (!SUPPORTED_AGENT_IDS.includes(agentId)) {
  console.error(`Usage: node integrations/agents/install.mjs <${SUPPORTED_AGENT_IDS.join("|")}>`);
  process.exitCode = 1;
} else {
  try {
    const installed = installHooks({ agentId });
    console.log(`AgentLight ${installed.name} hooks installed globally in ${installed.filePath}`);
    console.log(`Hook command: ${installed.command}`);
  } catch (error) {
    console.error(`Could not install AgentLight ${agentId} hooks: ${error.message}`);
    process.exitCode = 1;
  }
}
