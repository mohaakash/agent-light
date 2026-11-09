use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Manager, State};

use crate::core::{AppState, DisplayState};
use crate::outputs::{self, hardware};

#[tauri::command]
pub fn list_serial_ports() -> Result<Vec<hardware::SerialPortOption>, String> {
    hardware::available_ports()
}

#[tauri::command]
pub fn get_hardware_status(state: State<'_, AppState>) -> Result<hardware::HardwareStatus, String> {
    state.hardware.status()
}

#[tauri::command]
pub fn connect_serial_device(
    app: AppHandle,
    state: State<'_, AppState>,
    port_name: String,
) -> Result<hardware::HardwareStatus, String> {
    let current_state = state
        .display
        .lock()
        .map(|snapshot| snapshot.state)
        .map_err(|_| "display state lock is unavailable".to_string())?;
    let status = state.hardware.connect(&port_name, current_state)?;
    outputs::publish_hardware_status(&app, &status)?;
    Ok(status)
}

#[tauri::command]
pub fn disconnect_serial_device(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<hardware::HardwareStatus, String> {
    let status = state.hardware.disconnect()?;
    outputs::publish_hardware_status(&app, &status)?;
    Ok(status)
}

#[tauri::command]
pub fn test_hardware_led(
    app: AppHandle,
    state: State<'_, AppState>,
    display_state: DisplayState,
) -> Result<(), String> {
    if !state.hardware.status()?.connected {
        return Err("connect an AgentLight ESP8266 before testing LEDs".to_string());
    }
    let failure = state.hardware.send_status(display_state)?;
    if let Some(status) = failure {
        outputs::publish_hardware_status(&app, &status)?;
        return Err(status
            .error
            .unwrap_or_else(|| "hardware device disconnected".to_string()));
    }

    let app = app.clone();
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(2));
        let app_state = app.state::<AppState>();
        let current_state = match app_state.display.lock() {
            Ok(snapshot) => snapshot.state,
            Err(_) => return,
        };
        match app_state.hardware.send_status(current_state) {
            Ok(Some(status)) => {
                let _ = outputs::publish_hardware_status(&app, &status);
            }
            Ok(None) => {}
            Err(error) => eprintln!("AgentLight hardware output error: {error}"),
        }
    });
    Ok(())
}

pub fn start_hardware_heartbeat(app: AppHandle) {
    thread::Builder::new()
        .name("agentlight-hardware-heartbeat".to_string())
        .spawn(move || loop {
            thread::sleep(Duration::from_secs(5));
            let state = app.state::<AppState>();
            match state.hardware.heartbeat() {
                Ok(Some(status)) => {
                    let _ = outputs::publish_hardware_status(&app, &status);
                }
                Ok(None) => {}
                Err(error) => eprintln!("AgentLight hardware heartbeat error: {error}"),
            }
        })
        .expect("failed to start AgentLight hardware heartbeat");
}
