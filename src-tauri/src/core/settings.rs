use std::fs;

use tauri::{AppHandle, Manager, WebviewWindow};

use super::{AppSettings, WidgetPosition, WidgetSize};

const SETTINGS_FILE: &str = "settings.json";

pub fn load(app: &AppHandle) -> Result<AppSettings, String> {
    let path = app
        .path()
        .app_config_dir()
        .map_err(|error| format!("failed to resolve settings directory: {error}"))?
        .join(SETTINGS_FILE);

    match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map_err(|error| format!("failed to parse AgentLight settings: {error}")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(AppSettings::default()),
        Err(error) => Err(format!("failed to read AgentLight settings: {error}")),
    }
}

pub fn save(app: &AppHandle, settings: &AppSettings) -> Result<(), String> {
    let directory = app
        .path()
        .app_config_dir()
        .map_err(|error| format!("failed to resolve settings directory: {error}"))?;
    fs::create_dir_all(&directory)
        .map_err(|error| format!("failed to create settings directory: {error}"))?;

    let bytes = serde_json::to_vec_pretty(settings)
        .map_err(|error| format!("failed to encode AgentLight settings: {error}"))?;
    fs::write(directory.join(SETTINGS_FILE), bytes)
        .map_err(|error| format!("failed to save AgentLight settings: {error}"))
}

pub fn clamp_position(
    window: &WebviewWindow,
    position: &WidgetPosition,
    widget_size: WidgetSize,
) -> Option<WidgetPosition> {
    let monitors = window.available_monitors().ok()?;
    let monitor = monitors.iter().find(|monitor| {
        let origin = monitor.position();
        let size = monitor.size();
        position.x >= origin.x
            && position.y >= origin.y
            && position.x < origin.x.saturating_add(size.width as i32)
            && position.y < origin.y.saturating_add(size.height as i32)
    });

    let monitor = monitor.or_else(|| monitors.first())?;
    let origin = monitor.position();
    let size = monitor.size();
    let (width, height) = widget_size.dimensions();
    let scale_factor = window.scale_factor().unwrap_or(1.0);
    let width = (width as f64 * scale_factor).round() as i32;
    let height = (height as f64 * scale_factor).round() as i32;
    let max_x = origin
        .x
        .saturating_add(size.width as i32)
        .saturating_sub(width);
    let max_y = origin
        .y
        .saturating_add(size.height as i32)
        .saturating_sub(height);

    Some(WidgetPosition {
        x: position.x.clamp(origin.x, max_x.max(origin.x)),
        y: position.y.clamp(origin.y, max_y.max(origin.y)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_default_to_always_on_top_without_a_saved_position() {
        let settings = AppSettings::default();
        assert!(settings.traffic_light_always_on_top);
        assert!(settings.traffic_light_position.is_none());
    }

    #[test]
    fn settings_round_trip_through_json() {
        let settings = AppSettings {
            traffic_light_always_on_top: false,
            traffic_light_position: Some(WidgetPosition { x: 240, y: 180 }),
            ..AppSettings::default()
        };
        let encoded = serde_json::to_vec(&settings).unwrap();
        let decoded: AppSettings = serde_json::from_slice(&encoded).unwrap();

        assert_eq!(decoded.traffic_light_always_on_top, false);
        assert_eq!(decoded.traffic_light_position.unwrap().x, 240);
    }
}
