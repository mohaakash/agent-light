use tauri::State;

use crate::core::session::SessionSnapshot;
use crate::core::AppState;
use crate::core::DisplaySnapshot;

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
    let mut sessions = state
        .sessions
        .lock()
        .map_err(|_| "session state lock is unavailable".to_string())?;
    let now = std::time::Instant::now();
    sessions.tick(now);
    let state = sessions.state_for_agent(&agent);
    Ok(DisplaySnapshot::manual(state))
}
