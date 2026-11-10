use tauri::{AppHandle, Emitter, Manager, State};

use crate::core::settings;
use crate::core::{AppState, WidgetAppearance, WidgetSize, WidgetTheme};

pub const WIDGET_APPEARANCE_EVENT: &str = "widget-appearance-changed";

#[tauri::command]
pub fn set_agent_widget_visibility(
    app: AppHandle,
    agent: String,
    visible: bool,
) -> Result<(), String> {
    if ![
        "codex",
        "claude",
        "cursor",
        "gemini",
        "windsurf",
        "copilot",
        "antigravity",
        "pi",
    ]
    .contains(&agent.as_str())
    {
        return Err("unknown agent widget".to_string());
    }
    let label = format!("traffic-light-{agent}");
    let window = app
        .get_webview_window(&label)
        .ok_or_else(|| format!("{label} window is unavailable"))?;
    if visible {
        window
            .show()
            .map_err(|error| format!("failed to show {label}: {error}"))?;
    } else {
        window
            .hide()
            .map_err(|error| format!("failed to hide {label}: {error}"))?;
    }
    Ok(())
}

#[tauri::command]
pub fn start_agent_widget_drag(app: AppHandle, agent: String) -> Result<(), String> {
    if ![
        "codex",
        "claude",
        "cursor",
        "gemini",
        "windsurf",
        "copilot",
        "antigravity",
        "pi",
    ]
    .contains(&agent.as_str())
    {
        return Err("unknown agent widget".to_string());
    }
    let label = format!("traffic-light-{agent}");
    app.get_webview_window(&label)
        .ok_or_else(|| format!("{label} window is unavailable"))?
        .start_dragging()
        .map_err(|error| format!("failed to start dragging {label}: {error}"))
}

#[tauri::command]
pub fn get_widget_appearance(state: State<'_, AppState>) -> Result<WidgetAppearance, String> {
    state
        .settings
        .lock()
        .map(|current| WidgetAppearance {
            size: current.widget_size,
            theme: current.widget_theme,
            opacity: current.widget_opacity,
        })
        .map_err(|_| "widget appearance lock is unavailable".to_string())
}

#[tauri::command]
pub fn set_widget_appearance(
    app: AppHandle,
    state: State<'_, AppState>,
    size: WidgetSize,
    theme: WidgetTheme,
    opacity: u8,
) -> Result<WidgetAppearance, String> {
    if !(40..=100).contains(&opacity) {
        return Err("widget opacity must be between 40 and 100".to_string());
    }
    let window = app
        .get_webview_window("traffic-light")
        .ok_or_else(|| "traffic-light window is unavailable".to_string())?;
    let (width, height) = size.dimensions();
    window
        .set_size(tauri::LogicalSize::new(width, height))
        .map_err(|error| format!("failed to resize traffic-light window: {error}"))?;
    let adjusted_position = window.outer_position().ok().and_then(|position| {
        settings::clamp_position(
            &window,
            &crate::core::WidgetPosition {
                x: position.x,
                y: position.y,
            },
            size,
        )
    });
    if let Some(position) = &adjusted_position {
        window
            .set_position(tauri::PhysicalPosition::new(position.x, position.y))
            .map_err(|error| format!("failed to keep resized widget on screen: {error}"))?;
    }

    let appearance = WidgetAppearance {
        size,
        theme,
        opacity,
    };
    {
        let mut current = state
            .settings
            .lock()
            .map_err(|_| "widget appearance lock is unavailable".to_string())?;
        current.widget_size = size;
        current.widget_theme = theme;
        current.widget_opacity = opacity;
        if adjusted_position.is_some() {
            current.traffic_light_position = adjusted_position;
        }
        settings::save(&app, &current)?;
    }
    app.emit(WIDGET_APPEARANCE_EVENT, appearance)
        .map_err(|error| format!("failed to broadcast widget appearance: {error}"))?;
    Ok(appearance)
}

#[tauri::command]
pub fn set_traffic_light_visibility(app: AppHandle, visible: bool) -> Result<(), String> {
    let window = app
        .get_webview_window("traffic-light")
        .ok_or_else(|| "traffic-light window is unavailable".to_string())?;

    if visible {
        window
            .show()
            .map_err(|error| format!("failed to show traffic-light window: {error}"))?;
        window
            .set_focus()
            .map_err(|error| format!("failed to focus traffic-light window: {error}"))?;
    } else {
        window
            .hide()
            .map_err(|error| format!("failed to hide traffic-light window: {error}"))?;
    }

    Ok(())
}

#[tauri::command]
pub fn set_traffic_light_always_on_top(
    app: AppHandle,
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<(), String> {
    let window = app
        .get_webview_window("traffic-light")
        .ok_or_else(|| "traffic-light window is unavailable".to_string())?;

    window
        .set_always_on_top(enabled)
        .map_err(|error| format!("failed to update always-on-top: {error}"))?;

    state
        .settings
        .lock()
        .map_err(|_| "window preference lock is unavailable".to_string())
        .and_then(|mut current| {
            current.traffic_light_always_on_top = enabled;
            settings::save(&app, &current)
        })
}

#[tauri::command]
pub fn get_traffic_light_always_on_top(state: State<'_, AppState>) -> Result<bool, String> {
    state
        .settings
        .lock()
        .map(|settings| settings.traffic_light_always_on_top)
        .map_err(|_| "window preference lock is unavailable".to_string())
}
