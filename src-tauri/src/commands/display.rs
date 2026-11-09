use tauri::{AppHandle, State};

use crate::core::{AppState, DisplaySnapshot, DisplayState};
use crate::outputs::publish_display_state;

#[tauri::command]
pub fn get_display_snapshot(state: State<'_, AppState>) -> Result<DisplaySnapshot, String> {
    state
        .display
        .lock()
        .map(|snapshot| snapshot.clone())
        .map_err(|_| "display state lock is unavailable".to_string())
}

#[tauri::command]
pub fn set_manual_display_state(
    app: AppHandle,
    app_state: State<'_, AppState>,
    state: DisplayState,
) -> Result<DisplaySnapshot, String> {
    let snapshot = DisplaySnapshot::manual(state);

    app_state
        .display
        .lock()
        .map(|mut current| *current = snapshot.clone())
        .map_err(|_| "display state lock is unavailable".to_string())?;

    publish_display_state(&app, &snapshot)?;

    Ok(snapshot)
}
