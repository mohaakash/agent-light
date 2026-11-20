use std::process::Command;

use serde::Serialize;
use tauri::{Manager, State};

use crate::core::session::FocusTarget;
use crate::core::AppState;

#[derive(Clone, Debug, Serialize)]
pub struct FocusOutcome {
    /// True when an existing agent window was brought to the foreground.
    pub activated: bool,
    /// Machine-readable description of what happened, e.g. `window` or `folder`.
    pub method: String,
    /// Human-readable explanation shown in the dashboard or widget tooltip.
    pub detail: String,
}

#[tauri::command]
pub fn get_agent_focus_target(
    state: State<'_, AppState>,
    agent: String,
) -> Result<Option<FocusTarget>, String> {
    state
        .sessions
        .lock()
        .map(|sessions| sessions.focus_target(&agent))
        .map_err(|_| "session state lock is unavailable".to_string())
}

#[tauri::command]
pub fn focus_agent_session(app: tauri::AppHandle, agent: String) -> Result<FocusOutcome, String> {
    let target = app
        .state::<AppState>()
        .sessions
        .lock()
        .map_err(|_| "session state lock is unavailable".to_string())?
        .focus_target(&agent)
        .ok_or_else(|| format!("no tracked session is available for {agent}"))?;

    if let Some(pid) = target.process_id {
        if activate_process_window(pid) {
            return Ok(FocusOutcome {
                activated: true,
                method: "window".to_string(),
                detail: format!("Focused the {agent} window"),
            });
        }
    }

    if let Some(cwd) = target.cwd.as_deref() {
        if open_folder(cwd) {
            return Ok(FocusOutcome {
                activated: false,
                method: "folder".to_string(),
                detail: format!("Opened {cwd} because the agent window could not be focused"),
            });
        }
    }

    Err(match target.project {
        Some(project) => format!(
            "Could not focus {agent} for {project}. Start the agent again from a terminal so AgentLight can record its window."
        ),
        None => format!(
            "Could not focus {agent}. Start the agent again from a terminal so AgentLight can record its window."
        ),
    })
}

#[cfg(target_os = "windows")]
fn activate_process_window(pid: u32) -> bool {
    use windows_sys::core::BOOL;
    use windows_sys::Win32::Foundation::{HWND, LPARAM};
    use windows_sys::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        BringWindowToTop, EnumWindows, GetForegroundWindow, GetWindowThreadProcessId, IsIconic,
        IsWindowVisible, SetForegroundWindow, ShowWindow, SW_RESTORE,
    };

    struct Search {
        pid: u32,
        found: HWND,
    }

    unsafe extern "system" fn callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let search = &mut *(lparam as *mut Search);
        let mut window_pid = 0_u32;
        GetWindowThreadProcessId(hwnd, &mut window_pid);
        if window_pid == search.pid && IsWindowVisible(hwnd) != 0 {
            search.found = hwnd;
            return 0;
        }
        1
    }

    let mut search = Search {
        pid,
        found: std::ptr::null_mut(),
    };
    let enumerated = unsafe { EnumWindows(Some(callback), &mut search as *mut Search as LPARAM) };
    if enumerated == 0 && search.found.is_null() {
        return false;
    }
    if search.found.is_null() {
        return false;
    }

    unsafe {
        if IsIconic(search.found) != 0 {
            ShowWindow(search.found, SW_RESTORE);
        }
        let foreground = GetForegroundWindow();
        let foreground_thread = GetWindowThreadProcessId(foreground, std::ptr::null_mut());
        let target_thread = GetWindowThreadProcessId(search.found, std::ptr::null_mut());
        let current_thread = GetCurrentThreadId();
        let attached = foreground_thread != target_thread
            && AttachThreadInput(current_thread, target_thread, 1) != 0;
        BringWindowToTop(search.found);
        SetForegroundWindow(search.found);
        if attached {
            AttachThreadInput(current_thread, target_thread, 0);
        }
        GetForegroundWindow() == search.found
    }
}

#[cfg(target_os = "macos")]
fn activate_process_window(pid: u32) -> bool {
    Command::new("osascript")
        .args([
            "-e",
            &format!(
                "tell application \"System Events\" to set frontmost of (first process whose unix id is {pid}) to true"
            ),
        ])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(all(unix, not(target_os = "macos")))]
fn activate_process_window(pid: u32) -> bool {
    Command::new("xdotool")
        .args([
            "search",
            "--onlyvisible",
            "--pid",
            &pid.to_string(),
            "windowactivate",
        ])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(not(any(target_os = "windows", target_os = "macos", unix)))]
fn activate_process_window(_pid: u32) -> bool {
    false
}

fn open_folder(path: &str) -> bool {
    if !std::path::Path::new(path).is_dir() {
        return false;
    }
    #[cfg(target_os = "windows")]
    let command = Command::new("explorer").arg(path).spawn();
    #[cfg(target_os = "macos")]
    let command = Command::new("open").arg(path).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let command = Command::new("xdg-open").arg(path).spawn();
    #[cfg(not(any(target_os = "windows", target_os = "macos", unix)))]
    let command: Result<std::process::Child, std::io::Error> =
        Err(std::io::Error::other("unsupported platform"));

    command.map(|_| true).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::open_folder;

    #[test]
    fn open_folder_rejects_paths_that_do_not_exist() {
        assert!(!open_folder("this-path-should-not-exist-agentlight"));
    }
}
