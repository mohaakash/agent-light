use tauri::{AppHandle, Emitter, Manager, State};

use crate::core::settings;
use crate::core::{AppState, AudioAlertSettings};

pub const AUDIO_SETTINGS_EVENT: &str = "audio-settings-changed";

fn current_settings(state: &AppState) -> Result<AudioAlertSettings, String> {
    state
        .settings
        .lock()
        .map(|s| AudioAlertSettings {
            sound_enabled: s.sound_enabled,
            sound_volume: s.sound_volume,
            sound_on_attention: s.sound_on_attention,
            sound_on_completion: s.sound_on_completion,
            sound_preset: s.sound_preset,
            notifications_enabled: s.notifications_enabled,
            do_not_disturb: s.do_not_disturb,
            do_not_disturb_until: s.do_not_disturb_until.clone(),
            suppress_when_focused: s.suppress_when_focused,
        })
        .map_err(|_| "settings lock is unavailable".to_string())
}

#[tauri::command]
pub fn get_audio_settings(state: State<'_, AppState>) -> Result<AudioAlertSettings, String> {
    current_settings(&state)
}

#[tauri::command]
pub fn set_audio_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: AudioAlertSettings,
) -> Result<AudioAlertSettings, String> {
    if settings.sound_volume > 100 {
        return Err("sound volume must be between 0 and 100".to_string());
    }

    {
        let mut current = state
            .settings
            .lock()
            .map_err(|_| "settings lock is unavailable".to_string())?;
        current.sound_enabled = settings.sound_enabled;
        current.sound_volume = settings.sound_volume;
        current.sound_on_attention = settings.sound_on_attention;
        current.sound_on_completion = settings.sound_on_completion;
        current.sound_preset = settings.sound_preset;
        current.notifications_enabled = settings.notifications_enabled;
        current.do_not_disturb = settings.do_not_disturb;
        current.do_not_disturb_until = settings.do_not_disturb_until.clone();
        current.suppress_when_focused = settings.suppress_when_focused;
        settings::save(&app, &current)?;
    }

    app.emit(AUDIO_SETTINGS_EVENT, settings.clone())
        .map_err(|error| format!("failed to broadcast audio settings: {error}"))?;

    Ok(settings)
}

/// Flip the do-not-disturb master switch without going through the dashboard.
pub fn toggle_do_not_disturb(app: AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let mut next = current_settings(&state)?;
    let now_active = next.do_not_disturb || next.do_not_disturb_until.is_some();
    next.do_not_disturb = !now_active;
    next.do_not_disturb_until = None;
    set_audio_settings(app.clone(), state, next).map(|_| ())
}

/// Flip the sound master switch without going through the dashboard.
pub fn toggle_sound_enabled(app: AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let mut next = current_settings(&state)?;
    next.sound_enabled = !next.sound_enabled;
    set_audio_settings(app.clone(), state, next).map(|_| ())
}
