use std::fs;
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use uuid::Uuid;

use crate::core::session::{EventKind, SessionEvent};
use crate::core::{AppState, DisplaySnapshot};
use crate::outputs::publish_display_state;

pub const EVENT_PORT: u16 = 38_527;
const SCHEMA_VERSION: u8 = 1;
pub type CodexEventKind = EventKind;

#[derive(Clone, Debug, Deserialize)]
pub struct AgentEvent {
    pub schema_version: u8,
    pub event_id: String,
    pub agent: String,
    pub session_id: String,
    pub project: Option<String>,
    #[serde(rename = "event")]
    pub kind: CodexEventKind,
    pub occurred_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BridgeConfig {
    pub endpoint: String,
    pub token: String,
    pub schema_version: u8,
}

pub fn initialize_bridge(app: &AppHandle) -> Result<BridgeConfig, String> {
    let directory = app
        .path()
        .app_config_dir()
        .map_err(|error| format!("failed to resolve AgentLight config directory: {error}"))?;
    fs::create_dir_all(&directory)
        .map_err(|error| format!("failed to create AgentLight config directory: {error}"))?;

    let token_path = directory.join("api-token");
    let token = fs::read_to_string(&token_path)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    fs::write(&token_path, &token)
        .map_err(|error| format!("failed to write AgentLight API token: {error}"))?;

    let config = BridgeConfig {
        endpoint: format!("http://127.0.0.1:{EVENT_PORT}/api/v1/events"),
        token,
        schema_version: SCHEMA_VERSION,
    };

    let bridge_path = directory.join("codex-bridge.json");
    let encoded = serde_json::to_vec_pretty(&config)
        .map_err(|error| format!("failed to encode AgentLight bridge config: {error}"))?;
    fs::write(bridge_path, encoded)
        .map_err(|error| format!("failed to write AgentLight bridge config: {error}"))?;

    Ok(config)
}

pub fn apply_event(app: &AppHandle, event: AgentEvent) -> Result<(), String> {
    validate(&event)?;
    let app_state = app.state::<AppState>();
    let applied = app_state
        .sessions
        .lock()
        .map_err(|_| "session state lock is unavailable".to_string())?
        .apply(
            SessionEvent {
                event_id: event.event_id,
                agent: event.agent,
                session_id: event.session_id,
                project: event.project,
                kind: event.kind,
                occurred_at: event.occurred_at,
            },
            std::time::Instant::now(),
        )?;

    if applied {
        refresh_display_state(app, true)?;
    }
    Ok(())
}

pub fn start_session_maintenance(app: AppHandle) {
    thread::Builder::new()
        .name("agentlight-session-maintenance".to_string())
        .spawn(move || loop {
            thread::sleep(Duration::from_secs(1));
            if let Err(error) = refresh_display_state(&app, false) {
                eprintln!("AgentLight session state could not be refreshed: {error}");
            }
        })
        .expect("failed to start AgentLight session maintenance");
}

fn refresh_display_state(app: &AppHandle, force: bool) -> Result<(), String> {
    let app_state = app.state::<AppState>();
    let (state, changed) = {
        let mut sessions = app_state
            .sessions
            .lock()
            .map_err(|_| "session state lock is unavailable".to_string())?;
        let state = sessions.tick(std::time::Instant::now());
        (state, sessions.take_status_changed())
    };
    if !changed && !force {
        return Ok(());
    }
    let snapshot = DisplaySnapshot::codex(state);
    let should_publish = {
        let mut current = app_state
            .display
            .lock()
            .map_err(|_| "display state lock is unavailable".to_string())?;
        if *current == snapshot {
            false
        } else {
            *current = snapshot.clone();
            true
        }
    };
    if should_publish {
        publish_display_state(app, &snapshot)?;
    }
    Ok(())
}

fn validate(event: &AgentEvent) -> Result<(), String> {
    if event.schema_version != SCHEMA_VERSION
        || !matches!(
            event.agent.as_str(),
            "codex" | "claude" | "cursor" | "gemini" | "windsurf" | "copilot" | "antigravity"
        )
        || event.event_id.is_empty()
        || event.event_id.len() > 128
        || event.session_id.is_empty()
        || event.session_id.len() > 256
        || event
            .project
            .as_ref()
            .is_some_and(|project| project.len() > 128)
        || event
            .occurred_at
            .as_ref()
            .is_some_and(|occurred_at| occurred_at.len() > 64)
    {
        return Err("invalid AgentLight event".to_string());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{AgentEvent, CodexEventKind};

    fn event(kind: CodexEventKind) -> AgentEvent {
        AgentEvent {
            schema_version: 1,
            event_id: "event-1".to_string(),
            agent: "codex".to_string(),
            session_id: "session-1".to_string(),
            project: Some("agent-light".to_string()),
            kind,
            occurred_at: None,
        }
    }

    #[test]
    fn validates_canonical_codex_events() {
        assert!(super::validate(&event(CodexEventKind::UserInputRequired)).is_ok());
        let mut invalid = event(CodexEventKind::AgentWorking);
        invalid.schema_version = 2;
        assert!(super::validate(&invalid).is_err());
    }
}
