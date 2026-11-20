use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::core::session::EventKind;
use crate::integrations::hooks::{self, HookStatus};

const BRIDGE_HEALTH_TIMEOUT: Duration = Duration::from_millis(1200);

#[derive(Clone, Debug, Serialize)]
pub struct IntegrationDiagnostic {
    pub agent: String,
    pub installed: bool,
    pub config_path: String,
    pub config_exists: bool,
    pub bridge_valid: bool,
    pub status_text: String,
    pub backup_available: bool,
    pub backup_path: Option<String>,
    pub hook_command: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct BridgeVerification {
    pub reachable: bool,
    pub token_matches: bool,
    pub endpoint: String,
    pub detail: String,
}

#[derive(Clone, Debug, Deserialize)]
struct BridgeConfig {
    endpoint: String,
    token: String,
}

fn user_home_dir(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    app.path()
        .home_dir()
        .ok()
        .or_else(|| {
            std::env::var("USERPROFILE")
                .or_else(|_| std::env::var("HOME"))
                .ok()
                .map(std::path::PathBuf::from)
        })
        .ok_or_else(|| "the user home directory could not be resolved".to_string())
}

fn read_bridge_config(app: &AppHandle) -> Result<BridgeConfig, String> {
    let path = app
        .path()
        .app_config_dir()
        .map_err(|error| format!("failed to resolve the AgentLight config directory: {error}"))?
        .join("codex-bridge.json");
    let text = std::fs::read_to_string(&path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_str(&text)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

fn runtime_token(app: &AppHandle) -> Result<String, String> {
    let path = app
        .path()
        .app_config_dir()
        .map_err(|error| format!("failed to resolve the AgentLight config directory: {error}"))?
        .join("api-token");
    std::fs::read_to_string(&path)
        .map(|token| token.trim().to_string())
        .map_err(|error| format!("failed to read {}: {error}", path.display()))
}

/// Probe the loopback bridge without depending on an HTTP client crate.
fn probe_bridge_health(endpoint: &str) -> Result<bool, String> {
    let url = endpoint
        .strip_prefix("http://")
        .ok_or_else(|| "the bridge endpoint must use http://127.0.0.1".to_string())?;
    let authority = url.split('/').next().unwrap_or(url);
    let host = authority.split(':').next().unwrap_or("127.0.0.1");
    let port: u16 = authority
        .split(':')
        .nth(1)
        .unwrap_or("80")
        .parse()
        .map_err(|_| "the bridge endpoint port is invalid".to_string())?;

    let address: SocketAddr = (host, port)
        .to_socket_addrs()
        .map_err(|error| format!("failed to resolve the bridge address: {error}"))?
        .next()
        .ok_or_else(|| "the bridge address did not resolve".to_string())?;

    let mut stream = TcpStream::connect_timeout(&address, BRIDGE_HEALTH_TIMEOUT)
        .map_err(|error| format!("the AgentLight bridge is not reachable: {error}"))?;
    let _ = stream.set_read_timeout(Some(BRIDGE_HEALTH_TIMEOUT));
    let _ = stream.set_write_timeout(Some(BRIDGE_HEALTH_TIMEOUT));

    stream
        .write_all(
            format!("GET /api/v1/health HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n")
                .as_bytes(),
        )
        .map_err(|error| format!("failed to query the bridge health endpoint: {error}"))?;

    let mut response = String::new();
    let _ = stream.read_to_string(&mut response);
    Ok(response.starts_with("HTTP/1.1 200"))
}

#[tauri::command]
pub fn verify_bridge(app: AppHandle) -> Result<BridgeVerification, String> {
    let bridge = read_bridge_config(&app)?;
    let token_matches = runtime_token(&app)
        .map(|token| token == bridge.token)
        .unwrap_or(false);

    match probe_bridge_health(&bridge.endpoint) {
        Ok(true) => Ok(BridgeVerification {
            reachable: true,
            token_matches,
            endpoint: bridge.endpoint,
            detail: if token_matches {
                "The loopback bridge is healthy and the token matches.".to_string()
            } else {
                "The bridge responds, but the token in codex-bridge.json does not match the runtime secret. Reinstall the hooks to refresh it."
                    .to_string()
            },
        }),
        Ok(false) => Ok(BridgeVerification {
            reachable: false,
            token_matches,
            endpoint: bridge.endpoint,
            detail: "The bridge responded with an unexpected status.".to_string(),
        }),
        Err(error) => Ok(BridgeVerification {
            reachable: false,
            token_matches,
            endpoint: bridge.endpoint,
            detail: error,
        }),
    }
}

fn describe(status: HookStatus, bridge_valid: bool) -> &'static str {
    match status {
        HookStatus::NotInstalled => "Not installed",
        HookStatus::OutOfSync => "Out of sync",
        HookStatus::Misconfigured => "Misconfigured",
        HookStatus::Installed if bridge_valid => "Installed & healthy",
        HookStatus::Installed => "Installed (bridge offline)",
    }
}

#[tauri::command]
pub fn install_agent_integration(app: AppHandle, agent: String) -> Result<String, String> {
    if !hooks::is_supported_agent(&agent) {
        return Err(format!("Unsupported agent integration: {agent}"));
    }
    let home = user_home_dir(&app)?;
    let mutation = hooks::install(&agent, &home)?;
    let mut message = format!(
        "Updated {} hook entries in {}",
        mutation.changed_entries,
        mutation.config_path.display()
    );
    if let Some(backup) = &mutation.backup_path {
        message.push_str(&format!("\nBackup: {}", backup.display()));
    }
    for note in mutation.notes {
        message.push_str(&format!("\n{note}"));
    }
    Ok(message)
}

#[tauri::command]
pub fn uninstall_agent_integration(app: AppHandle, agent: String) -> Result<String, String> {
    if !hooks::is_supported_agent(&agent) {
        return Err(format!("Unsupported agent integration: {agent}"));
    }
    let home = user_home_dir(&app)?;
    let mutation = hooks::uninstall(&agent, &home)?;
    if mutation.changed_entries == 0 {
        return Ok(format!(
            "No AgentLight hooks were present in {}",
            mutation.config_path.display()
        ));
    }
    Ok(format!(
        "Removed {} AgentLight hook entries from {}",
        mutation.changed_entries,
        mutation.config_path.display()
    ))
}

#[tauri::command]
pub fn restore_agent_integration(app: AppHandle, agent: String) -> Result<String, String> {
    if !hooks::is_supported_agent(&agent) {
        return Err(format!("Unsupported agent integration: {agent}"));
    }
    let home = user_home_dir(&app)?;
    match hooks::restore_backup(&agent, &home)? {
        Some(path) => Ok(format!(
            "Restored the original configuration at {}",
            path.display()
        )),
        None => Err("No AgentLight backup exists for this agent yet.".to_string()),
    }
}

#[tauri::command]
pub fn get_agent_integration_status(app: AppHandle, agent: String) -> Result<bool, String> {
    if !hooks::is_supported_agent(&agent) {
        return Err(format!("Unsupported agent integration: {agent}"));
    }
    let home = user_home_dir(&app)?;
    Ok(hooks::status(&agent, &home).is_installed())
}

#[tauri::command]
pub fn get_agent_integration_diagnostic(
    app: AppHandle,
    agent: String,
) -> Result<IntegrationDiagnostic, String> {
    if !hooks::is_supported_agent(&agent) {
        return Err(format!("Unsupported agent integration: {agent}"));
    }
    let home = user_home_dir(&app)?;
    let config_path = hooks::config_path(&home, &agent)
        .ok_or_else(|| format!("Unsupported agent integration: {agent}"))?;
    let status = hooks::status(&agent, &home);
    let bridge_valid = app
        .path()
        .app_config_dir()
        .map(|dir| dir.join("codex-bridge.json").is_file())
        .unwrap_or(false);

    let hook_command = hooks::hook_command(&agent);
    Ok(IntegrationDiagnostic {
        agent,
        installed: status.is_installed(),
        config_path: config_path.to_string_lossy().to_string(),
        config_exists: config_path.is_file(),
        bridge_valid,
        status_text: describe(status, bridge_valid).to_string(),
        backup_available: hooks::backup_available(&config_path),
        backup_path: hooks::backup_available(&config_path).then(|| {
            hooks::backup_path(&config_path)
                .to_string_lossy()
                .to_string()
        }),
        hook_command,
    })
}

#[tauri::command]
pub fn send_test_agent_event(
    app: AppHandle,
    agent: String,
    event_type: String,
) -> Result<(), String> {
    let supported = matches!(
        agent.as_str(),
        "codex" | "claude" | "cursor" | "gemini" | "windsurf" | "copilot" | "antigravity"
    );
    if !supported {
        return Err(format!("Unsupported agent integration: {agent}"));
    }

    let kind = match event_type.to_lowercase().as_str() {
        "permission_required" | "attention" | "waiting" => EventKind::PermissionRequired,
        "agent_working" | "working" => EventKind::AgentWorking,
        "task_completed" | "completed" => EventKind::TaskCompleted,
        "session_idle" | "idle" => EventKind::SessionIdle,
        "session_error" | "error" => EventKind::SessionError,
        _ => return Err(format!("Unknown test event type: {event_type}")),
    };

    let event = crate::integrations::codex::AgentEvent {
        schema_version: 1,
        event_id: uuid::Uuid::new_v4().to_string(),
        agent: agent.clone(),
        session_id: format!("test-{agent}-session"),
        project: Some("Test Workspace".to_string()),
        kind,
        occurred_at: Some(chrono::Utc::now().to_rfc3339()),
        cwd: None,
        client_app: None,
        process_id: None,
    };

    crate::integrations::codex::apply_event(&app, event)
}
