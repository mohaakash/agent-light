use std::path::Path;
use std::process::Command;

fn run_node_script(script: &str, args: Vec<&str>) -> Result<String, String> {
    let project_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or_else(|| "AgentLight project root is unavailable".to_string())?;
    let script_path = project_root.join(script);
    if !script_path.is_file() {
        return Err(format!(
            "Integration script is missing: {}",
            script_path.display()
        ));
    }

    let output = Command::new("node")
        .arg(&script_path)
        .args(args)
        .current_dir(project_root)
        .output()
        .map_err(|error| format!("Could not start Node.js: {error}"))?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
    }
    let details = String::from_utf8_lossy(&output.stderr).trim().to_string();
    Err(if details.is_empty() {
        format!("Integration script exited with {}", output.status)
    } else {
        details
    })
}

#[tauri::command]
pub fn install_agent_integration(agent: String) -> Result<String, String> {
    let (script, script_args): (&str, Vec<&str>) = match agent.as_str() {
        "codex" => ("integrations/codex/install.mjs", Vec::new()),
        "claude" => ("integrations/claude/install.mjs", Vec::new()),
        "cursor" | "gemini" | "windsurf" | "copilot" | "antigravity" => {
            ("integrations/agents/install.mjs", vec![agent.as_str()])
        }
        _ => return Err(format!("Unsupported agent integration: {agent}")),
    };

    let message = run_node_script(script, script_args)?;
    Ok(if message.is_empty() {
        format!("{agent} integration installed globally")
    } else {
        message
    })
}

#[tauri::command]
pub fn get_agent_integration_status(agent: String) -> Result<bool, String> {
    let supported = matches!(
        agent.as_str(),
        "codex" | "claude" | "cursor" | "gemini" | "windsurf" | "copilot" | "antigravity"
    );
    if !supported {
        return Err(format!("Unsupported agent integration: {agent}"));
    }
    Ok(
        run_node_script("integrations/status.mjs", vec![agent.as_str()])?
            .eq_ignore_ascii_case("true"),
    )
}
