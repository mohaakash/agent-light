use tauri::State;

use crate::core::display_state::DisplaySource;
use crate::core::session::{SessionManager, SessionSnapshot};
use crate::core::{AppState, DisplaySnapshot};

#[tauri::command]
pub fn get_sessions(state: State<'_, AppState>) -> Result<Vec<SessionSnapshot>, String> {
    let sessions = state
        .sessions
        .lock()
        .map_err(|_| "session state lock is unavailable".to_string())?;
    Ok(sessions.snapshots(std::time::Instant::now()))
}

#[tauri::command]
pub fn get_agent_display_snapshot(
    state: State<'_, AppState>,
    agent: String,
) -> Result<DisplaySnapshot, String> {
    let global_snapshot = state
        .display
        .lock()
        .map(|snapshot| snapshot.clone())
        .map_err(|_| "display state lock is unavailable".to_string())?;
    let mut sessions = state
        .sessions
        .lock()
        .map_err(|_| "session state lock is unavailable".to_string())?;
    Ok(resolve_agent_display_snapshot(
        global_snapshot,
        &mut sessions,
        &agent,
    ))
}

fn resolve_agent_display_snapshot(
    global_snapshot: DisplaySnapshot,
    sessions: &mut SessionManager,
    agent: &str,
) -> DisplaySnapshot {
    if global_snapshot.source == DisplaySource::ManualPreview {
        return global_snapshot;
    }
    sessions.tick(std::time::Instant::now());
    DisplaySnapshot::manual(sessions.state_for_agent(agent))
}

#[cfg(test)]
mod tests {
    use super::resolve_agent_display_snapshot;
    use crate::core::session::SessionManager;
    use crate::core::{DisplaySnapshot, DisplayState};

    #[test]
    fn manual_preview_is_shared_with_agent_widgets() {
        let mut sessions = SessionManager::default();
        let snapshot = resolve_agent_display_snapshot(
            DisplaySnapshot::manual(DisplayState::Working),
            &mut sessions,
            "codex",
        );
        assert_eq!(snapshot, DisplaySnapshot::manual(DisplayState::Working));
    }
}
