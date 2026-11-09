pub mod hardware;

use tauri::{AppHandle, Emitter, Manager};

use crate::core::{AppState, DisplaySnapshot};

pub const DISPLAY_STATE_EVENT: &str = "display-state-changed";

pub fn publish_display_state(app: &AppHandle, snapshot: &DisplaySnapshot) -> Result<(), String> {
    app.emit(DISPLAY_STATE_EVENT, snapshot)
        .map_err(|error| format!("failed to broadcast display state: {error}"))?;

    let state = app.state::<AppState>();
    match state.hardware.send_status(snapshot.state) {
        Ok(Some(status)) => {
            let _ = publish_hardware_status(app, &status);
        }
        Ok(None) => {}
        Err(error) => eprintln!("AgentLight hardware output error: {error}"),
    }
    Ok(())
}

pub const HARDWARE_STATUS_EVENT: &str = "hardware-status-changed";

pub fn publish_hardware_status(
    app: &AppHandle,
    status: &hardware::HardwareStatus,
) -> Result<(), String> {
    app.emit(HARDWARE_STATUS_EVENT, status)
        .map_err(|error| format!("failed to broadcast hardware status: {error}"))
}
