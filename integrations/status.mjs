import { isHooksInstalled as isCodexInstalled } from "./codex/hooks-lib.mjs";
import { isHooksInstalled as isClaudeInstalled } from "./claude/hooks-lib.mjs";
import { isHooksInstalled as isAgentInstalled } from "./agents/hooks-lib.mjs";

const agentId = process.argv[2];
let installed = false;
try {
  installed = agentId === "codex"
    ? isCodexInstalled()
    : agentId === "claude"
      ? isClaudeInstalled()
      : isAgentInstalled({ agentId });
} catch {
  installed = false;
}
process.stdout.write(`${installed}\n`);
