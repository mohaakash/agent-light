use serde::{Deserialize, Serialize};
use tauri::{
    AppHandle, Emitter, Manager, Monitor, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};

use crate::core::settings;
use crate::core::{
    AppState, FacePack, HotZone, HotZonePreset, MonitorRect, NormalizedHotZone, WidgetAppearance,
    WidgetSize, WidgetTheme, WidgetVisualStyle,
};

pub const WIDGET_APPEARANCE_EVENT: &str = "widget-appearance-changed";
pub const HOTZONE_MODE_EVENT: &str = "hotzone-mode-changed";
pub const DISPLAY_TOPOLOGY_EVENT: &str = "display-topology-changed";
pub const HOTZONE_WIDGET_EVENT: &str = "hotzone-widget-visibility-changed";

#[derive(Clone, Debug, Serialize)]
pub struct HotzoneWidgetVisibility {
    pub agent: String,
    pub visible: bool,
    pub anchor: String,
    pub hotzone: HotZone,
}

const AGENT_WIDGETS: &[&str] = &[
    "codex",
    "claude",
    "cursor",
    "gemini",
    "windsurf",
    "copilot",
    "antigravity",
    "pi",
];

const TRAFFIC_LIGHT_LABELS: &[&str] = &[
    "traffic-light-codex",
    "traffic-light-claude",
    "traffic-light-cursor",
    "traffic-light-gemini",
    "traffic-light-windsurf",
    "traffic-light-copilot",
    "traffic-light-antigravity",
    "traffic-light-pi",
];

pub fn ensure_agent_widget_window(app: &AppHandle, agent: &str) -> Result<WebviewWindow, String> {
    if !AGENT_WIDGETS.contains(&agent) {
        return Err("unknown agent widget".to_string());
    }

    let label = format!("traffic-light-{agent}");
    if let Some(window) = app.get_webview_window(&label) {
        return Ok(window);
    }

    let state = app.state::<AppState>();
    let (size, visual_style, always_on_top, position) = state
        .settings
        .lock()
        .map(|settings| {
            (
                settings.widget_size,
                settings.widget_visual_style,
                settings.traffic_light_always_on_top,
                settings
                    .agent_widget_positions
                    .get(agent)
                    .cloned()
                    .or_else(|| settings.traffic_light_position.clone()),
            )
        })
        .map_err(|_| "settings lock is unavailable".to_string())?;
    let (width, height) = size.dimensions_for(visual_style);
    let url = format!("index.html?window=traffic-light&agent={agent}");
    let window = WebviewWindowBuilder::new(app, &label, WebviewUrl::App(url.into()))
        .title(format!("AgentLight · {agent}"))
        .inner_size(width as f64, height as f64)
        .decorations(false)
        .transparent(true)
        .always_on_top(always_on_top)
        .skip_taskbar(true)
        .resizable(false)
        .visible(false)
        .focusable(true)
        .closable(false)
        .shadow(false)
        .build()
        .map_err(|error| format!("failed to create {label}: {error}"))?;

    if let Some(position) = position
        .and_then(|position| settings::clamp_position(&window, &position, size, visual_style))
    {
        window
            .set_position(tauri::PhysicalPosition::new(position.x, position.y))
            .map_err(|error| format!("failed to position {label}: {error}"))?;
    }
    Ok(window)
}

#[tauri::command]
pub fn get_agent_widget_visibility(
    state: State<'_, AppState>,
    agent: String,
) -> Result<bool, String> {
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
    state
        .settings
        .lock()
        .map(|settings| {
            settings
                .agent_widget_visibility
                .get(&agent)
                .copied()
                .unwrap_or(false)
        })
        .map_err(|_| "settings lock is unavailable".to_string())
}

#[tauri::command]
pub fn set_agent_widget_visibility(
    app: AppHandle,
    state: State<'_, AppState>,
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

    let hotzone_mode = {
        let mut settings = state
            .settings
            .lock()
            .map_err(|_| "settings lock is unavailable".to_string())?;
        settings
            .agent_widget_visibility
            .insert(agent.clone(), visible);
        settings::save(&app, &settings)?;
        settings.hotzone_mode
    };

    let label = format!("traffic-light-{agent}");
    if visible {
        let window = ensure_agent_widget_window(&app, &agent)?;
        if hotzone_mode {
            window
                .hide()
                .map_err(|error| format!("failed to hide {label}: {error}"))?;
        } else {
            window
                .show()
                .map_err(|error| format!("failed to show {label}: {error}"))?;
        }
    } else if let Some(window) = app.get_webview_window(&label) {
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
            visual_style: current.widget_visual_style,
            face_pack: current.widget_face_pack,
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
    visual_style: WidgetVisualStyle,
    face_pack: FacePack,
) -> Result<WidgetAppearance, String> {
    if !(40..=100).contains(&opacity) {
        return Err("widget opacity must be between 40 and 100".to_string());
    }
    let (width, height) = size.dimensions_for(visual_style);
    // Track the last adjusted position for saving (use the first window found)
    let mut last_adjusted_position: Option<crate::core::WidgetPosition> = None;
    // Resize all traffic light windows
    for label in TRAFFIC_LIGHT_LABELS {
        if let Some(w) = app.get_webview_window(label) {
            w.set_size(tauri::LogicalSize::new(width, height))
                .map_err(|error| format!("failed to resize {label}: {error}"))?;
            if let Some(position) = w.outer_position().ok().and_then(|pos| {
                settings::clamp_position(
                    &w,
                    &crate::core::WidgetPosition { x: pos.x, y: pos.y },
                    size,
                    visual_style,
                )
            }) {
                w.set_position(tauri::PhysicalPosition::new(position.x, position.y))
                    .map_err(|error| format!("failed to keep {label} on screen: {error}"))?;
                if last_adjusted_position.is_none() {
                    last_adjusted_position = Some(position);
                }
            }
        }
    }

    let appearance = WidgetAppearance {
        size,
        theme,
        opacity,
        visual_style,
        face_pack,
    };
    {
        let mut current = state
            .settings
            .lock()
            .map_err(|_| "widget appearance lock is unavailable".to_string())?;
        current.widget_size = size;
        current.widget_theme = theme;
        current.widget_opacity = opacity;
        current.widget_visual_style = visual_style;
        current.widget_face_pack = face_pack;
        if let Some(pos) = last_adjusted_position {
            current.traffic_light_position = Some(pos);
        }
        settings::save(&app, &current)?;
    }
    app.emit(WIDGET_APPEARANCE_EVENT, appearance)
        .map_err(|error| format!("failed to broadcast widget appearance: {error}"))?;
    Ok(appearance)
}

#[tauri::command]
pub fn set_traffic_light_visibility(
    app: AppHandle,
    state: State<'_, AppState>,
    visible: bool,
) -> Result<(), String> {
    let (enabled_agents, hotzone_mode) = state
        .settings
        .lock()
        .map(|settings| {
            (
                settings.agent_widget_visibility.clone(),
                settings.hotzone_mode,
            )
        })
        .map_err(|_| "settings lock is unavailable".to_string())?;

    for label in TRAFFIC_LIGHT_LABELS {
        if let Some(window) = app.get_webview_window(label) {
            let agent = label.strip_prefix("traffic-light-").unwrap_or(label);
            if visible && !hotzone_mode && enabled_agents.get(agent).copied().unwrap_or(false) {
                window
                    .show()
                    .map_err(|error| format!("failed to show {label}: {error}"))?;
            } else {
                window
                    .hide()
                    .map_err(|error| format!("failed to hide {label}: {error}"))?;
            }
        }
    }
    Ok(())
}

#[tauri::command]
pub fn set_traffic_light_always_on_top(
    app: AppHandle,
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<(), String> {
    for label in TRAFFIC_LIGHT_LABELS {
        if let Some(window) = app.get_webview_window(label) {
            window
                .set_always_on_top(enabled)
                .map_err(|error| format!("failed to update always-on-top for {label}: {error}"))?;
        }
    }

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

// Hotzone mode commands
#[tauri::command]
pub fn get_hotzone_mode(state: State<'_, AppState>) -> Result<bool, String> {
    state
        .settings
        .lock()
        .map(|settings| settings.hotzone_mode)
        .map_err(|_| "settings lock is unavailable".to_string())
}

#[tauri::command]
pub fn set_hotzone_mode(
    app: AppHandle,
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<(), String> {
    let (enabled_agents, agent_positions, default_position, widget_size, visual_style) = state
        .settings
        .lock()
        .map(|settings| {
            (
                settings.agent_widget_visibility.clone(),
                settings.agent_widget_positions.clone(),
                settings.traffic_light_position.clone(),
                settings.widget_size,
                settings.widget_visual_style,
            )
        })
        .map_err(|_| "settings lock is unavailable".to_string())?;

    {
        let mut current = state
            .settings
            .lock()
            .map_err(|_| "settings lock is unavailable".to_string())?;
        current.hotzone_mode = enabled;
        settings::save(&app, &current)?;
    }

    for label in TRAFFIC_LIGHT_LABELS {
        if let Some(window) = app.get_webview_window(label) {
            let agent = label.strip_prefix("traffic-light-").unwrap_or(label);
            let should_show = !enabled && enabled_agents.get(agent).copied().unwrap_or(false);
            if should_show {
                if let Some(position) = agent_positions
                    .get(agent)
                    .or(default_position.as_ref())
                    .and_then(|position| {
                        settings::clamp_position(&window, position, widget_size, visual_style)
                    })
                {
                    window
                        .set_position(tauri::PhysicalPosition::new(position.x, position.y))
                        .map_err(|error| format!("failed to restore {label} position: {error}"))?;
                }
                window
                    .show()
                    .map_err(|error| format!("failed to show {label}: {error}"))?;
            } else {
                window
                    .hide()
                    .map_err(|error| format!("failed to hide {label}: {error}"))?;
            }
        }
    }

    app.emit(HOTZONE_MODE_EVENT, enabled)
        .map_err(|error| format!("failed to broadcast hotzone mode change: {error}"))?;
    Ok(())
}

// Per-agent hotzone commands

/// Live display geometry used to resolve normalized hotzones.
pub fn monitor_rects(app: &AppHandle) -> Vec<MonitorRect> {
    app.available_monitors()
        .map(|monitors| {
            monitors
                .iter()
                .map(|monitor| {
                    let position = monitor.position();
                    let size = monitor.size();
                    MonitorRect {
                        x: position.x,
                        y: position.y,
                        width: size.width,
                        height: size.height,
                        scale_factor: monitor.scale_factor(),
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Resolve an agent's hotzone, preferring the normalized definition so it
/// follows monitor moves, resolution changes, and DPI scaling.
fn resolve_agent_hotzone(
    settings: &crate::core::AppSettings,
    monitors: &[MonitorRect],
    agent: &str,
) -> Option<HotZone> {
    settings
        .agent_hotzone_presets
        .get(agent)
        .and_then(|preset| preset.resolve(monitors))
        .or_else(|| settings.agent_hotzones.get(agent).cloned())
}

#[tauri::command]
pub fn get_agent_hotzone(
    app: AppHandle,
    state: State<'_, AppState>,
    agent: String,
) -> Result<Option<HotZone>, String> {
    let monitors = monitor_rects(&app);
    state
        .settings
        .lock()
        .map(|settings| resolve_agent_hotzone(&settings, &monitors, &agent))
        .map_err(|_| "settings lock is unavailable".to_string())
}

#[tauri::command]
pub fn get_agent_hotzone_preset(
    state: State<'_, AppState>,
    agent: String,
) -> Result<Option<NormalizedHotZone>, String> {
    state
        .settings
        .lock()
        .map(|settings| settings.agent_hotzone_presets.get(&agent).cloned())
        .map_err(|_| "settings lock is unavailable".to_string())
}

#[tauri::command]
pub fn set_agent_hotzone_preset(
    app: AppHandle,
    state: State<'_, AppState>,
    agent: String,
    preset: NormalizedHotZone,
) -> Result<(), String> {
    if !AGENT_WIDGETS.contains(&agent.as_str()) {
        return Err("unknown agent widget".to_string());
    }
    if !(50..=1000).contains(&preset.size_logical) {
        return Err("hotzone size must be between 50 and 1000 logical pixels".to_string());
    }
    let monitors = monitor_rects(&app);
    if monitors.get(preset.monitor_index).is_none() {
        return Err(format!("monitor {} is not connected", preset.monitor_index));
    }
    let resolved = preset
        .resolve(&monitors)
        .ok_or_else(|| "the hotzone could not be resolved on this display".to_string())?;

    {
        let mut current = state
            .settings
            .lock()
            .map_err(|_| "settings lock is unavailable".to_string())?;
        current.agent_hotzone_presets.insert(agent.clone(), preset);
        current.agent_hotzones.insert(agent, resolved);
        settings::save(&app, &current)?;
    }
    Ok(())
}

#[tauri::command]
pub fn set_agent_hotzone(
    app: AppHandle,
    state: State<'_, AppState>,
    agent: String,
    hotzone: HotZone,
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

    {
        let mut current = state
            .settings
            .lock()
            .map_err(|_| "settings lock is unavailable".to_string())?;
        // An absolute save supersedes any stored normalized definition.
        current.agent_hotzone_presets.remove(&agent);
        current.agent_hotzones.insert(agent, hotzone);
        settings::save(&app, &current)?;
    }

    Ok(())
}

// Hotzone collision detection - checks all hotzones (default + agent)
#[tauri::command]
pub fn get_active_hotzone_at(
    app: AppHandle,
    state: State<'_, AppState>,
    x: i32,
    y: i32,
) -> Result<Option<HotZone>, String> {
    let monitors = monitor_rects(&app);
    state
        .settings
        .lock()
        .map(|settings| {
            // Check default hotzone first
            if settings.default_hotzone.contains(x, y) {
                return Some(settings.default_hotzone.clone());
            }
            let agents: Vec<String> = settings
                .agent_hotzones
                .keys()
                .chain(settings.agent_hotzone_presets.keys())
                .cloned()
                .collect();
            for agent in agents {
                if let Some(hotzone) = resolve_agent_hotzone(&settings, &monitors, &agent) {
                    if hotzone.contains(x, y) {
                        return Some(hotzone);
                    }
                }
            }
            None
        })
        .map_err(|_| "settings lock is unavailable".to_string())
}

// Check if a point is in the default hotzone (for main traffic light)
#[tauri::command]
pub fn get_default_hotzone(state: State<'_, AppState>) -> Result<HotZone, String> {
    state
        .settings
        .lock()
        .map(|settings| settings.default_hotzone.clone())
        .map_err(|_| "settings lock is unavailable".to_string())
}

// Check if a point is in a specific agent's hotzone (for agent traffic lights)
#[tauri::command]
pub fn get_agent_hotzone_at(
    app: AppHandle,
    state: State<'_, AppState>,
    agent: String,
    x: i32,
    y: i32,
) -> Result<bool, String> {
    let monitors = monitor_rects(&app);
    state
        .settings
        .lock()
        .map(|settings| {
            resolve_agent_hotzone(&settings, &monitors, &agent)
                .map(|hotzone| hotzone.contains(x, y))
                .unwrap_or(false)
        })
        .map_err(|_| "settings lock is unavailable".to_string())
}

// --- Multi-monitor support ---

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MonitorInfo {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
    pub name: Option<String>,
}

impl From<&Monitor> for MonitorInfo {
    fn from(m: &Monitor) -> Self {
        let pos = m.position();
        let size = m.size();
        Self {
            x: pos.x,
            y: pos.y,
            width: size.width,
            height: size.height,
            scale_factor: m.scale_factor(),
            name: m.name().cloned(),
        }
    }
}

/// Get all available monitors with their dimensions and positions
#[tauri::command]
pub fn get_available_monitors(app: AppHandle) -> Result<Vec<MonitorInfo>, String> {
    let monitors = app
        .available_monitors()
        .map_err(|error| format!("failed to get monitors: {error}"))?;
    Ok(monitors.iter().map(MonitorInfo::from).collect())
}

/// Get which monitor contains a point (x, y). Returns the monitor index.
#[tauri::command]
pub fn get_monitor_at(app: AppHandle, x: i32, y: i32) -> Result<Option<usize>, String> {
    let monitors = app
        .available_monitors()
        .map_err(|error| format!("failed to get monitors: {error}"))?;
    for (i, monitor) in monitors.iter().enumerate() {
        let pos = monitor.position();
        let size = monitor.size();
        if x >= pos.x
            && x < pos.x.saturating_add(size.width as i32)
            && y >= pos.y
            && y < pos.y.saturating_add(size.height as i32)
        {
            return Ok(Some(i));
        }
    }
    Ok(None)
}

// --- Peek delay configuration ---

#[tauri::command]
pub fn get_peek_delay(state: State<'_, AppState>) -> Result<u32, String> {
    state
        .settings
        .lock()
        .map(|settings| settings.peek_delay_ms)
        .map_err(|_| "settings lock is unavailable".to_string())
}

#[tauri::command]
pub fn set_peek_delay(
    app: AppHandle,
    state: State<'_, AppState>,
    delay_ms: u32,
) -> Result<(), String> {
    if delay_ms < 100 || delay_ms > 10000 {
        return Err("peek delay must be between 100 and 10000 ms".to_string());
    }
    {
        let mut current = state
            .settings
            .lock()
            .map_err(|_| "settings lock is unavailable".to_string())?;
        current.peek_delay_ms = delay_ms;
        settings::save(&app, &current)?;
    }
    Ok(())
}

// --- Hotzone presets ---

/// Get available hotzone preset names
#[tauri::command]
pub fn get_hotzone_presets() -> Result<Vec<String>, String> {
    Ok(vec![
        "top_left".to_string(),
        "top_right".to_string(),
        "bottom_left".to_string(),
        "bottom_right".to_string(),
        "center_top".to_string(),
        "center_bottom".to_string(),
        "center_left".to_string(),
        "center_right".to_string(),
    ])
}

/// Calculate a preset hotzone position relative to a monitor of given dimensions
#[tauri::command]
pub fn get_preset_hotzone(
    preset: String,
    monitor_width: u32,
    monitor_height: u32,
    hotzone_size: u32,
) -> Result<HotZone, String> {
    let preset_enum = match preset.as_str() {
        "top_left" => HotZonePreset::TopLeft,
        "top_right" => HotZonePreset::TopRight,
        "bottom_left" => HotZonePreset::BottomLeft,
        "bottom_right" => HotZonePreset::BottomRight,
        "center_top" => HotZonePreset::CenterTop,
        "center_bottom" => HotZonePreset::CenterBottom,
        "center_left" => HotZonePreset::CenterLeft,
        "center_right" => HotZonePreset::CenterRight,
        _ => return Err(format!("unknown preset: {preset}")),
    };
    let (x, y) = preset_enum.position(monitor_width, monitor_height, hotzone_size);
    Ok(HotZone {
        x,
        y,
        width: hotzone_size,
        height: hotzone_size,
        monitor_id: None,
    })
}

// --- Default hotzone configuration ---

#[tauri::command]
pub fn set_default_hotzone(
    app: AppHandle,
    state: State<'_, AppState>,
    hotzone: HotZone,
) -> Result<(), String> {
    {
        let mut current = state
            .settings
            .lock()
            .map_err(|_| "settings lock is unavailable".to_string())?;
        current.default_hotzone = hotzone;
        settings::save(&app, &current)?;
    }
    Ok(())
}
