//! Native hook configuration management for every supported coding agent.
//!
//! This replaces the previous Node.js installer scripts (`integrations/*/install.mjs`)
//! so hook installation works in packaged builds without a Node.js runtime on PATH
//! for the configuration step. Every write is atomic and takes a one-time backup of
//! the user's existing configuration before the first modification.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Map, Value};
use uuid::Uuid;

const BACKUP_SUFFIX: &str = ".agentlight-backup";
const HOOK_MARKER: &str = "integrations/agents/hook.mjs";
const CODEX_HOOK_SCRIPT: &str = "integrations/codex/hook.mjs";
const CLAUDE_HOOK_SCRIPT: &str = "integrations/claude/hook.mjs";
const AGENTS_HOOK_SCRIPT: &str = "integrations/agents/hook.mjs";

pub const AGENT_IDS: [&str; 7] = [
    "codex",
    "claude",
    "cursor",
    "gemini",
    "windsurf",
    "copilot",
    "antigravity",
];

const CODEX_EVENTS: [&str; 8] = [
    "SessionStart",
    "SessionEnd",
    "UserPromptSubmit",
    "PermissionRequest",
    "PreToolUse",
    "PostToolUse",
    "Stop",
    "Interrupt",
];

const CLAUDE_EVENTS: [&str; 8] = [
    "SessionStart",
    "SessionEnd",
    "UserPromptSubmit",
    "PermissionRequest",
    "PreToolUse",
    "PostToolUse",
    "Stop",
    "StopFailure",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ConfigKind {
    Cursor,
    Nested,
    Flat,
    Copilot,
    Antigravity,
}

struct AgentConfig {
    relative_path: &'static [&'static str],
    kind: ConfigKind,
    events: &'static [&'static str],
}

const CURSOR_EVENTS: [&str; 11] = [
    "sessionStart",
    "sessionEnd",
    "beforeSubmitPrompt",
    "preToolUse",
    "postToolUse",
    "postToolUseFailure",
    "subagentStart",
    "subagentStop",
    "preCompact",
    "afterAgentThought",
    "stop",
];

const GEMINI_EVENTS: [&str; 8] = [
    "SessionStart",
    "SessionEnd",
    "BeforeAgent",
    "AfterAgent",
    "BeforeTool",
    "AfterTool",
    "Notification",
    "PreCompress",
];

const WINDSURF_EVENTS: [&str; 12] = [
    "pre_read_code",
    "post_read_code",
    "pre_write_code",
    "post_write_code",
    "pre_run_command",
    "post_run_command",
    "pre_mcp_tool_use",
    "post_mcp_tool_use",
    "pre_user_prompt",
    "post_cascade_response",
    "post_cascade_response_with_transcript",
    "post_setup_worktree",
];

const COPILOT_EVENTS: [&str; 11] = [
    "sessionStart",
    "sessionEnd",
    "userPromptSubmitted",
    "preToolUse",
    "postToolUse",
    "errorOccurred",
    "agentStop",
    "subagentStart",
    "subagentStop",
    "preCompact",
    "permissionRequest",
];

const ANTIGRAVITY_EVENTS: [&str; 5] = [
    "SessionStart",
    "PreInvocation",
    "PreToolUse",
    "PostToolUse",
    "Stop",
];

fn agent_config(agent: &str) -> Option<AgentConfig> {
    let config = match agent {
        "cursor" => AgentConfig {
            relative_path: &[".cursor", "hooks.json"],
            kind: ConfigKind::Cursor,
            events: &CURSOR_EVENTS,
        },
        "gemini" => AgentConfig {
            relative_path: &[".gemini", "settings.json"],
            kind: ConfigKind::Nested,
            events: &GEMINI_EVENTS,
        },
        "windsurf" => AgentConfig {
            relative_path: &[".codeium", "windsurf", "hooks.json"],
            kind: ConfigKind::Flat,
            events: &WINDSURF_EVENTS,
        },
        "copilot" => AgentConfig {
            relative_path: &[".copilot", "hooks", "hooks.json"],
            kind: ConfigKind::Copilot,
            events: &COPILOT_EVENTS,
        },
        "antigravity" => AgentConfig {
            relative_path: &[".gemini", "config", "hooks.json"],
            kind: ConfigKind::Antigravity,
            events: &ANTIGRAVITY_EVENTS,
        },
        _ => return None,
    };
    Some(config)
}

/// Outcome of an install or restore operation.
#[derive(Clone, Debug)]
pub struct HookMutation {
    pub config_path: PathBuf,
    pub backup_path: Option<PathBuf>,
    pub changed_entries: usize,
    pub notes: Vec<String>,
}

/// High-level installation state used for the Integration Center badges.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HookStatus {
    NotInstalled,
    Installed,
    OutOfSync,
    Misconfigured,
}

impl HookStatus {
    pub fn is_installed(self) -> bool {
        self == Self::Installed
    }
}

pub fn is_supported_agent(agent: &str) -> bool {
    AGENT_IDS.contains(&agent)
}

/// Resolve a bundled integration script, preferring the packaged application
/// location over development-only manifest paths.
pub fn find_script(script: &str) -> Result<PathBuf, String> {
    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(dir) = exe_path.parent() {
            candidates.push(dir.join(script));
            candidates.push(dir.join("resources").join(script));
            candidates.push(dir.join("resources").join("_up_").join(script));
            candidates.push(dir.join("..").join("Resources").join(script));
        }
    }
    if let Some(manifest) = option_env!("CARGO_MANIFEST_DIR") {
        if let Some(parent) = Path::new(manifest).parent() {
            candidates.push(parent.join(script));
        }
    }
    candidates.push(Path::new(".").join(script));

    for candidate in candidates {
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    Err(format!(
        "AgentLight integration script is missing: {script}. Reinstall AgentLight to restore it."
    ))
}

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Locate a Node.js executable so hook commands keep working even when the
/// interactive PATH used by an IDE differs from the user's shell PATH.
fn node_executable() -> String {
    let exe_name = if cfg!(windows) { "node.exe" } else { "node" };
    if let Some(paths) = std::env::var_os("PATH") {
        for directory in std::env::split_paths(&paths) {
            let candidate = directory.join(exe_name);
            if candidate.is_file() {
                return normalize_path(&candidate);
            }
        }
    }
    "node".to_string()
}

fn codex_hook_command() -> Result<String, String> {
    let script = find_script(CODEX_HOOK_SCRIPT)?;
    Ok(format!("node \"{}\"", normalize_path(&script)))
}

fn claude_hook_command() -> Result<String, String> {
    let script = find_script(CLAUDE_HOOK_SCRIPT)?;
    Ok(format!("node \"{}\"", normalize_path(&script)))
}

fn agent_hook_command(agent: &str, event: &str) -> Result<String, String> {
    if !is_supported_agent(agent) || agent == "codex" || agent == "claude" {
        return Err(format!("Unsupported AgentLight adapter: {agent}"));
    }
    let script = find_script(AGENTS_HOOK_SCRIPT)?;
    let command = format!(
        "\"{}\" \"{}\" {agent}",
        node_executable(),
        normalize_path(&script)
    );
    Ok(if event.is_empty() {
        command
    } else {
        format!("{command} {event}")
    })
}

pub fn config_path(home: &Path, agent: &str) -> Option<PathBuf> {
    match agent {
        "codex" => Some(home.join(".codex").join("hooks.json")),
        "claude" => Some(home.join(".claude").join("settings.json")),
        _ => agent_config(agent).map(|config| {
            config
                .relative_path
                .iter()
                .fold(home.to_path_buf(), |path, segment| path.join(segment))
        }),
    }
}

fn codex_features_path(home: &Path) -> PathBuf {
    home.join(".codex").join("config.toml")
}

pub fn backup_path(config: &Path) -> PathBuf {
    let mut name = config.as_os_str().to_os_string();
    name.push(BACKUP_SUFFIX);
    PathBuf::from(name)
}

pub fn backup_available(config: &Path) -> bool {
    backup_path(config).is_file()
}

fn read_json_object(path: &Path) -> Result<Map<String, Value>, String> {
    if !path.is_file() {
        return Ok(Map::new());
    }
    let text = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    if text.trim().is_empty() {
        return Ok(Map::new());
    }
    let value: Value = serde_json::from_str(&text)
        .map_err(|error| format!("{} is not valid JSON: {error}", path.display()))?;
    match value {
        Value::Object(map) => Ok(map),
        _ => Err(format!(
            "{} must contain a JSON object at the top level",
            path.display()
        )),
    }
}

fn write_json_atomic(path: &Path, value: &Value) -> Result<(), String> {
    let mut encoded = serde_json::to_string_pretty(value)
        .map_err(|error| format!("failed to encode {}: {error}", path.display()))?;
    encoded.push('\n');
    write_text_atomic(path, &encoded)
}

fn write_text_atomic(path: &Path, contents: &str) -> Result<(), String> {
    if let Some(directory) = path.parent() {
        fs::create_dir_all(directory)
            .map_err(|error| format!("failed to create {}: {error}", directory.display()))?;
    }
    let directory = path.parent().unwrap_or(Path::new("."));
    let temporary = directory.join(format!(".agentlight-{}.tmp", Uuid::new_v4()));
    fs::write(&temporary, contents)
        .map_err(|error| format!("failed to write {}: {error}", temporary.display()))?;

    match fs::rename(&temporary, path) {
        Ok(()) => Ok(()),
        Err(_) => {
            // Windows cannot atomically replace an existing file through
            // `std::fs::rename`, so fall back to an overwriting copy.
            let result = fs::copy(&temporary, path)
                .map(|_| ())
                .map_err(|error| format!("failed to replace {}: {error}", path.display()));
            let _ = fs::remove_file(&temporary);
            result
        }
    }
}

fn ensure_backup(config: &Path) -> Result<Option<PathBuf>, String> {
    if !config.is_file() {
        return Ok(None);
    }
    let backup = backup_path(config);
    if backup.is_file() {
        return Ok(Some(backup));
    }
    fs::copy(config, &backup).map_err(|error| {
        format!(
            "failed to back up {} before changing it: {error}",
            config.display()
        )
    })?;
    Ok(Some(backup))
}

fn managed_text(value: &Value) -> bool {
    let text = match value {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    };
    text.contains(HOOK_MARKER)
        || text.contains("integrations/codex/hook.mjs")
        || text.contains("integrations/claude/hook.mjs")
        || text.to_ascii_lowercase().contains("agentlight")
}

fn remove_managed_entries(entries: Value) -> Vec<Value> {
    let Value::Array(items) = entries else {
        return Vec::new();
    };
    let mut kept = Vec::new();
    for mut entry in items {
        if let Value::Object(map) = &mut entry {
            if let Some(Value::Array(hooks)) = map.get("hooks").cloned() {
                let remaining: Vec<Value> = hooks
                    .into_iter()
                    .filter(|hook| !managed_text(hook))
                    .collect();
                if remaining.is_empty() {
                    continue;
                }
                map.insert("hooks".to_string(), Value::Array(remaining));
                kept.push(entry);
                continue;
            }
        }
        if !managed_text(&entry) {
            kept.push(entry);
        }
    }
    kept
}

fn hooks_map<'a>(
    config: &'a mut Map<String, Value>,
    path: &Path,
) -> Result<&'a mut Map<String, Value>, String> {
    match config.get("hooks") {
        Some(Value::Object(_)) => {}
        Some(_) => {
            return Err(format!(
                "{} has a 'hooks' value that is not an object",
                path.display()
            ))
        }
        None => {
            config.insert("hooks".to_string(), Value::Object(Map::new()));
        }
    }
    config
        .get_mut("hooks")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| format!("{} has an unusable 'hooks' object", path.display()))
}

fn event_entries(
    hooks: &Map<String, Value>,
    event: &str,
    path: &Path,
) -> Result<Vec<Value>, String> {
    match hooks.get(event) {
        None => Ok(Vec::new()),
        Some(Value::Array(entries)) => Ok(entries.clone()),
        Some(_) => Err(format!(
            "{} has a 'hooks.{event}' value that is not an array",
            path.display()
        )),
    }
}

fn contains_command(entries: &[Value], command: &str) -> bool {
    entries.iter().any(|entry| match entry {
        Value::Object(map) => {
            map.get("command").and_then(Value::as_str) == Some(command)
                || map.get("bash").and_then(Value::as_str) == Some(command)
                || map.get("powershell").and_then(Value::as_str) == Some(command)
                || map
                    .get("hooks")
                    .and_then(Value::as_array)
                    .is_some_and(|hooks| contains_command(hooks, command))
        }
        _ => false,
    })
}

// --- Codex -----------------------------------------------------------------

fn install_codex(home: &Path) -> Result<HookMutation, String> {
    let hooks_path = config_path(home, "codex")
        .ok_or_else(|| "codex configuration path is unavailable".to_string())?;
    let command = codex_hook_command()?;
    let mut config = read_json_object(&hooks_path)?;
    let backup = ensure_backup(&hooks_path)?;
    let mut updated = 0;

    {
        let hooks = hooks_map(&mut config, &hooks_path)?;
        for event in CODEX_EVENTS {
            let mut groups = event_entries(hooks, event, &hooks_path)?;
            let mut definition = Map::new();
            definition.insert("type".to_string(), json!("command"));
            definition.insert("command".to_string(), json!(command));
            if cfg!(windows) {
                definition.insert("commandWindows".to_string(), json!(command));
            }
            definition.insert("timeout".to_string(), json!(3));
            if event != "SessionEnd" {
                definition.insert("async".to_string(), json!(true));
            }

            let mut replaced = false;
            for group in groups.iter_mut() {
                let Some(group_map) = group.as_object_mut() else {
                    continue;
                };
                if let Some(Value::Array(nested)) = group_map.get_mut("hooks") {
                    for hook in nested.iter_mut() {
                        let Some(hook_map) = hook.as_object_mut() else {
                            continue;
                        };
                        if hook_map.get("type").and_then(Value::as_str) == Some("command")
                            && hook_map.get("command").and_then(Value::as_str)
                                == Some(command.as_str())
                        {
                            *hook_map = definition.clone();
                            replaced = true;
                        }
                    }
                }
            }

            if !replaced {
                groups.push(json!({ "hooks": [Value::Object(definition)] }));
            }
            hooks.insert(event.to_string(), Value::Array(groups));
            updated += 1;
        }
    }

    write_json_atomic(&hooks_path, &Value::Object(config))?;

    let features_path = codex_features_path(home);
    let mut notes = Vec::new();
    if ensure_codex_hooks_enabled(&features_path)? {
        notes.push("Enabled the Codex `hooks` feature flag in config.toml".to_string());
    }

    Ok(HookMutation {
        config_path: hooks_path,
        backup_path: backup,
        changed_entries: updated,
        notes,
    })
}

fn ensure_codex_hooks_enabled(config_path: &Path) -> Result<bool, String> {
    let existing = fs::read_to_string(config_path).unwrap_or_default();
    if config_path.is_file() {
        ensure_backup(config_path)?;
    }

    if existing.trim().is_empty() {
        write_text_atomic(config_path, "[features]\nhooks = true\n")?;
        return Ok(true);
    }

    let newline = if existing.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut lines: Vec<String> = existing
        .split('\n')
        .map(|line| line.trim_end_matches('\r').to_string())
        .collect();
    if lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }

    let is_section = |line: &str| {
        let trimmed = line.trim();
        trimmed.starts_with('[') && trimmed.ends_with(']')
    };

    if let Some(features_at) = lines.iter().position(|line| line.trim() == "[features]") {
        let mut section_end = lines.len();
        for (index, line) in lines.iter().enumerate().skip(features_at + 1) {
            if is_section(line) {
                section_end = index;
                break;
            }
        }
        for line in lines.iter().take(section_end).skip(features_at + 1) {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("hooks") {
                let rest = rest.trim_start();
                let Some(value) = rest.strip_prefix('=') else {
                    continue;
                };
                let value = value.trim();
                if value == "true" {
                    return Ok(false);
                }
                if value == "false" {
                    let indentation = line.len() - line.trim_start().len();
                    let replacement = format!("{}hooks = true", " ".repeat(indentation));
                    let mut updated = lines.clone();
                    if let Some(target) = updated
                        .iter_mut()
                        .take(section_end)
                        .skip(features_at + 1)
                        .find(|candidate| candidate.trim().starts_with("hooks"))
                    {
                        *target = replacement;
                        write_text_atomic(
                            config_path,
                            &format!("{}{newline}", updated.join(newline)),
                        )?;
                        return Ok(true);
                    }
                }
            }
        }
        lines.insert(features_at + 1, "hooks = true".to_string());
        write_text_atomic(config_path, &format!("{}{newline}", lines.join(newline)))?;
        return Ok(true);
    }

    let trimmed = existing.trim_end();
    write_text_atomic(
        config_path,
        &format!("{trimmed}{newline}{newline}[features]{newline}hooks = true{newline}"),
    )?;
    Ok(true)
}

// --- Claude ----------------------------------------------------------------

fn install_claude(home: &Path) -> Result<HookMutation, String> {
    let settings_path = config_path(home, "claude")
        .ok_or_else(|| "claude configuration path is unavailable".to_string())?;
    let command = claude_hook_command()?;
    let mut config = read_json_object(&settings_path)?;
    let backup = ensure_backup(&settings_path)?;
    let mut updated = 0;

    {
        let hooks = hooks_map(&mut config, &settings_path)?;
        for event in CLAUDE_EVENTS {
            let mut groups = event_entries(hooks, event, &settings_path)?;
            if !contains_command(&groups, &command) {
                groups.push(json!({
                    "hooks": [{ "type": "command", "command": command, "async": true, "timeout": 3 }]
                }));
                updated += 1;
            }
            hooks.insert(event.to_string(), Value::Array(groups));
        }
    }

    write_json_atomic(&settings_path, &Value::Object(config))?;
    Ok(HookMutation {
        config_path: settings_path,
        backup_path: backup,
        changed_entries: updated,
        notes: Vec::new(),
    })
}

// --- Cursor, Gemini, Windsurf, Copilot, Antigravity ------------------------

fn install_generic_agent(agent: &str, home: &Path) -> Result<HookMutation, String> {
    let config =
        agent_config(agent).ok_or_else(|| format!("Unsupported AgentLight adapter: {agent}"))?;
    let config_path = config_path(home, agent)
        .ok_or_else(|| format!("{agent} configuration path is unavailable"))?;
    let mut document = read_json_object(&config_path)?;
    let backup = ensure_backup(&config_path)?;
    let mut updated = 0;

    let entries_for = |event: &str| -> Result<Vec<Value>, String> {
        let command = agent_hook_command(agent, event)?;
        Ok(match config.kind {
            ConfigKind::Cursor | ConfigKind::Flat => {
                let mut entry = Map::new();
                entry.insert("command".to_string(), json!(command));
                if config.kind == ConfigKind::Flat {
                    entry.insert("show_output".to_string(), json!(false));
                }
                vec![Value::Object(entry)]
            }
            ConfigKind::Nested => vec![json!({
                "matcher": "*",
                "hooks": [{ "type": "command", "command": command, "name": "agentlight", "timeout": 30 }]
            })],
            ConfigKind::Copilot => vec![json!({
                "type": "command",
                "bash": command,
                "powershell": command,
                "timeoutSec": if event == "permissionRequest" { 600 } else { 5 }
            })],
            ConfigKind::Antigravity => Vec::new(),
        })
    };

    if config.kind == ConfigKind::Antigravity {
        let mut agent_light = document
            .get("agentlight")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        agent_light.insert("enabled".to_string(), json!(true));
        for event in config.events {
            let handler = json!({
                "type": "command",
                "command": agent_hook_command(agent, event)?,
                "timeout": 3
            });
            let value = if *event == "PreToolUse" || *event == "PostToolUse" {
                json!([{ "matcher": "*", "hooks": [handler] }])
            } else {
                json!([handler])
            };
            agent_light.insert(event.to_string(), value);
            updated += 1;
        }
        document.insert("agentlight".to_string(), Value::Object(agent_light));
    } else {
        let hooks = hooks_map(&mut document, &config_path)?;
        for event in config.events {
            let existing = match hooks.get(*event) {
                Some(Value::Array(entries)) => entries.clone(),
                Some(_) => {
                    return Err(format!(
                        "{} has a 'hooks.{event}' value that is not an array",
                        config_path.display()
                    ))
                }
                None => Vec::new(),
            };
            let mut entries = remove_managed_entries(Value::Array(existing));
            entries.extend(entries_for(event)?);
            hooks.insert(event.to_string(), Value::Array(entries));
            updated += 1;
        }
    }

    write_json_atomic(&config_path, &Value::Object(document))?;
    Ok(HookMutation {
        config_path,
        backup_path: backup,
        changed_entries: updated,
        notes: Vec::new(),
    })
}

// --- Public API ------------------------------------------------------------

pub fn install(agent: &str, home: &Path) -> Result<HookMutation, String> {
    match agent {
        "codex" => install_codex(home),
        "claude" => install_claude(home),
        _ => install_generic_agent(agent, home),
    }
}

/// The command AgentLight injects for an agent, used for diagnostics and support.
pub fn hook_command(agent: &str) -> Option<String> {
    match agent {
        "codex" => codex_hook_command().ok(),
        "claude" => claude_hook_command().ok(),
        _ => agent_hook_command(agent, "").ok(),
    }
}

pub fn uninstall(agent: &str, home: &Path) -> Result<HookMutation, String> {
    let config_path = config_path(home, agent)
        .ok_or_else(|| format!("Unsupported AgentLight adapter: {agent}"))?;
    if !config_path.is_file() {
        return Ok(HookMutation {
            config_path,
            backup_path: None,
            changed_entries: 0,
            notes: Vec::new(),
        });
    }
    let mut document = read_json_object(&config_path)?;
    let mut removed = 0;
    let mut notes = Vec::new();

    match agent {
        "codex" | "claude" => {
            let hooks = hooks_map(&mut document, &config_path)?;
            let events: Vec<String> = hooks.keys().cloned().collect();
            for event in events {
                let Value::Array(groups) = hooks.get(&event).cloned().unwrap_or(Value::Null) else {
                    continue;
                };
                let mut kept = Vec::new();
                for mut group in groups {
                    let Some(group_map) = group.as_object_mut() else {
                        kept.push(group);
                        continue;
                    };
                    if let Some(Value::Array(nested)) = group_map.get_mut("hooks") {
                        let before = nested.len();
                        nested.retain(|hook| !managed_text(hook));
                        removed += before - nested.len();
                        if nested.is_empty() {
                            continue;
                        }
                    }
                    kept.push(group);
                }
                if kept.is_empty() {
                    hooks.remove(&event);
                } else {
                    hooks.insert(event, Value::Array(kept));
                }
            }
        }
        "antigravity" => {
            if document.remove("agentlight").is_some() {
                removed = ANTIGRAVITY_EVENTS.len();
            }
        }
        _ => {
            let config = agent_config(agent)
                .ok_or_else(|| format!("Unsupported AgentLight adapter: {agent}"))?;
            let hooks = hooks_map(&mut document, &config_path)?;
            for event in config.events {
                let Some(existing) = hooks.get(*event).cloned() else {
                    continue;
                };
                let before = existing.as_array().map(Vec::len).unwrap_or(0);
                let entries = remove_managed_entries(existing);
                removed += before - entries.len();
                if entries.is_empty() {
                    hooks.remove(*event);
                } else {
                    hooks.insert(event.to_string(), Value::Array(entries));
                }
            }
        }
    }

    if removed > 0 {
        write_json_atomic(&config_path, &Value::Object(document))?;
        notes.push(format!("Removed {removed} AgentLight hook entries"));
    }

    Ok(HookMutation {
        config_path,
        backup_path: None,
        changed_entries: removed,
        notes,
    })
}

fn restore_file(target: &Path) -> Result<bool, String> {
    let backup = backup_path(target);
    if !backup.is_file() {
        return Ok(false);
    }
    let contents = fs::read(&backup)
        .map_err(|error| format!("failed to read {}: {error}", backup.display()))?;
    if let Some(directory) = target.parent() {
        fs::create_dir_all(directory)
            .map_err(|error| format!("failed to create {}: {error}", directory.display()))?;
    }
    fs::write(target, contents)
        .map_err(|error| format!("failed to restore {}: {error}", target.display()))?;
    Ok(true)
}

/// Restore the configuration captured the first time AgentLight modified it.
pub fn restore_backup(agent: &str, home: &Path) -> Result<Option<PathBuf>, String> {
    let config_path = config_path(home, agent)
        .ok_or_else(|| format!("Unsupported AgentLight adapter: {agent}"))?;
    if !restore_file(&config_path)? {
        return Ok(None);
    }
    if agent == "codex" {
        // The Codex adapter also enables the hooks feature flag in config.toml.
        restore_file(&codex_features_path(home))?;
    }
    Ok(Some(config_path))
}

fn codex_feature_enabled(home: &Path) -> bool {
    let path = codex_features_path(home);
    let Ok(text) = fs::read_to_string(path) else {
        return false;
    };
    let mut in_features = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_features = trimmed == "[features]";
            continue;
        }
        if in_features {
            if let Some(rest) = trimmed.strip_prefix("hooks") {
                if let Some(value) = rest.trim_start().strip_prefix('=') {
                    if value.trim() == "true" {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// Determine whether the managed hook entries still match the current
/// installation. A mismatch is reported as `OutOfSync` so the UI can offer a
/// one-click repair instead of silently reporting success.
pub fn status(agent: &str, home: &Path) -> HookStatus {
    if !is_supported_agent(agent) {
        return HookStatus::Misconfigured;
    }
    let Some(config_path) = config_path(home, agent) else {
        return HookStatus::Misconfigured;
    };
    if !config_path.is_file() {
        return HookStatus::NotInstalled;
    }
    let Ok(document) = read_json_object(&config_path) else {
        return HookStatus::Misconfigured;
    };

    let expected: Vec<(&str, String)> = match agent {
        "codex" => {
            let Ok(command) = codex_hook_command() else {
                return HookStatus::Misconfigured;
            };
            CODEX_EVENTS
                .iter()
                .map(|event| (*event, command.clone()))
                .collect()
        }
        "claude" => {
            let Ok(command) = claude_hook_command() else {
                return HookStatus::Misconfigured;
            };
            CLAUDE_EVENTS
                .iter()
                .map(|event| (*event, command.clone()))
                .collect()
        }
        _ => {
            let Some(config) = agent_config(agent) else {
                return HookStatus::Misconfigured;
            };
            let mut expected = Vec::new();
            for event in config.events {
                let Ok(command) = agent_hook_command(agent, event) else {
                    return HookStatus::Misconfigured;
                };
                expected.push((*event, command));
            }
            expected
        }
    };

    let read_entries = |event: &str| -> Option<Vec<Value>> {
        if agent == "antigravity" {
            let table = document.get("agentlight")?.as_object()?;
            if table.get("enabled").and_then(Value::as_bool) == Some(false) {
                return Some(Vec::new());
            }
            table.get(event)?.as_array().cloned()
        } else {
            document.get("hooks")?.get(event)?.as_array().cloned()
        }
    };

    let mut any_managed = false;
    let mut missing = false;
    let mut stale = false;

    for (event, command) in &expected {
        let entries = read_entries(event).unwrap_or_default();
        if entries.iter().any(managed_text) {
            any_managed = true;
        }
        if contains_command(&entries, command) {
            continue;
        }
        if entries.iter().any(managed_text) {
            stale = true;
        } else {
            missing = true;
        }
    }

    if agent == "codex" && !codex_feature_enabled(home) {
        return if any_managed {
            HookStatus::Misconfigured
        } else {
            HookStatus::NotInstalled
        };
    }

    if missing {
        HookStatus::NotInstalled
    } else if stale {
        HookStatus::OutOfSync
    } else {
        HookStatus::Installed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    struct TempHome(PathBuf);

    impl TempHome {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!("agentlight-test-{}", Uuid::new_v4()));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempHome {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn read(path: &Path) -> Value {
        serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
    }

    #[test]
    fn codex_install_preserves_user_hooks_and_creates_backup() {
        let home = TempHome::new();
        let codex_dir = home.path().join(".codex");
        fs::create_dir_all(&codex_dir).unwrap();
        let hooks_path = codex_dir.join("hooks.json");
        fs::write(
            &hooks_path,
            r#"{"hooks":{"Stop":[{"hooks":[{"type":"command","command":"my-own-hook"}]}]}}"#,
        )
        .unwrap();

        let mutation = install("codex", home.path()).unwrap();
        assert_eq!(mutation.changed_entries, CODEX_EVENTS.len());
        assert!(mutation.backup_path.unwrap().is_file());

        let config = read(&hooks_path);
        let stop_groups = config["hooks"]["Stop"].as_array().unwrap();
        let commands: Vec<&str> = stop_groups
            .iter()
            .flat_map(|group| group["hooks"].as_array().unwrap())
            .filter_map(|hook| hook["command"].as_str())
            .collect();
        assert!(commands.contains(&"my-own-hook"));
        assert!(commands.iter().any(|command| command.contains("hook.mjs")));
        assert!(codex_features_path(home.path()).is_file());
        assert_eq!(status("codex", home.path()), HookStatus::Installed);
    }

    #[test]
    fn codex_install_is_idempotent_and_updates_the_feature_flag() {
        let home = TempHome::new();
        let codex_dir = home.path().join(".codex");
        fs::create_dir_all(&codex_dir).unwrap();
        fs::write(codex_dir.join("config.toml"), "[features]\nhooks = false\n").unwrap();

        install("codex", home.path()).unwrap();
        let features = fs::read_to_string(codex_dir.join("config.toml")).unwrap();
        assert!(features.contains("hooks = true"));

        let first = read(&codex_dir.join("hooks.json"));
        install("codex", home.path()).unwrap();
        let second = read(&codex_dir.join("hooks.json"));
        assert_eq!(
            first, second,
            "reinstalling must not duplicate hook entries"
        );
    }

    #[test]
    fn codex_status_detects_out_of_sync_and_missing_config() {
        let home = TempHome::new();
        let codex_dir = home.path().join(".codex");
        fs::create_dir_all(&codex_dir).unwrap();
        install("codex", home.path()).unwrap();

        let hooks_path = codex_dir.join("hooks.json");
        let mut config = read(&hooks_path);
        config["hooks"]["Stop"][0]["hooks"][0]["command"] =
            json!("node \"/old/location/integrations/codex/hook.mjs\"");
        fs::write(&hooks_path, serde_json::to_vec_pretty(&config).unwrap()).unwrap();
        assert_eq!(status("codex", home.path()), HookStatus::OutOfSync);

        fs::remove_file(codex_dir.join("config.toml")).unwrap();
        assert_eq!(status("codex", home.path()), HookStatus::Misconfigured);
    }

    #[test]
    fn codex_uninstall_removes_only_managed_entries() {
        let home = TempHome::new();
        install("codex", home.path()).unwrap();
        let mutation = uninstall("codex", home.path()).unwrap();
        assert!(mutation.changed_entries >= CODEX_EVENTS.len());
        assert_eq!(status("codex", home.path()), HookStatus::NotInstalled);
    }

    #[test]
    fn claude_install_is_idempotent() {
        let home = TempHome::new();
        install("claude", home.path()).unwrap();
        let settings_path = home.path().join(".claude").join("settings.json");
        let first = read(&settings_path);
        install("claude", home.path()).unwrap();
        let second = read(&settings_path);
        assert_eq!(first, second);
        assert_eq!(status("claude", home.path()), HookStatus::Installed);
    }

    #[test]
    fn cursor_install_writes_flat_command_entries() {
        let home = TempHome::new();
        install("cursor", home.path()).unwrap();
        let config = read(&home.path().join(".cursor").join("hooks.json"));
        let command = config["hooks"]["stop"][0]["command"].as_str().unwrap();
        assert!(command.contains("integrations/agents/hook.mjs"));
        assert!(command.contains("cursor stop"));
        assert_eq!(status("cursor", home.path()), HookStatus::Installed);
        uninstall("cursor", home.path()).unwrap();
        assert_eq!(status("cursor", home.path()), HookStatus::NotInstalled);
    }

    #[test]
    fn antigravity_install_writes_and_removes_managed_block() {
        let home = TempHome::new();
        install("antigravity", home.path()).unwrap();
        let path = home
            .path()
            .join(".gemini")
            .join("config")
            .join("hooks.json");
        let config = read(&path);
        assert_eq!(config["agentlight"]["enabled"], json!(true));
        assert!(config["agentlight"]["PreToolUse"][0]["hooks"].is_array());

        uninstall("antigravity", home.path()).unwrap();
        let config = read(&path);
        assert!(config.get("agentlight").is_none());
    }

    #[test]
    fn restore_backup_returns_the_original_configuration() {
        let home = TempHome::new();
        let codex_dir = home.path().join(".codex");
        fs::create_dir_all(&codex_dir).unwrap();
        let hooks_path = codex_dir.join("hooks.json");
        let features_path = codex_dir.join("config.toml");
        let original =
            r#"{"hooks":{"Stop":[{"hooks":[{"type":"command","command":"my-own-hook"}]}]}}"#;
        fs::write(&hooks_path, original).unwrap();
        fs::write(&features_path, "model = \"gpt-5\"\n").unwrap();

        install("codex", home.path()).unwrap();
        assert_ne!(fs::read_to_string(&hooks_path).unwrap(), original);
        assert!(fs::read_to_string(&features_path)
            .unwrap()
            .contains("hooks = true"));

        assert!(restore_backup("codex", home.path()).unwrap().is_some());
        assert_eq!(fs::read_to_string(&hooks_path).unwrap(), original);
        assert_eq!(
            fs::read_to_string(&features_path).unwrap(),
            "model = \"gpt-5\"\n"
        );
    }
}
