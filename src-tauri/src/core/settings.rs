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
    visual_style: crate::core::WidgetVisualStyle,
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
    let (width, height) = widget_size.dimensions_for(visual_style);
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
    use crate::core::HotZone;
    use std::collections::HashMap;

    #[test]
    fn settings_default_to_always_on_top_without_a_saved_position() {
        let settings = AppSettings::default();
        assert!(settings.traffic_light_always_on_top);
        assert!(settings.traffic_light_position.is_none());
        assert!(!settings.hotzone_mode);
        assert!(settings.agent_hotzones.is_empty());
        assert!(settings.agent_widget_visibility.is_empty());
        assert_eq!(settings.default_hotzone.x, 0);
        assert_eq!(settings.default_hotzone.y, 0);
        assert_eq!(settings.default_hotzone.width, 100);
        assert_eq!(settings.default_hotzone.height, 100);
        assert_eq!(settings.peek_delay_ms, 2000);
        assert_eq!(
            settings.widget_visual_style,
            crate::core::WidgetVisualStyle::TrafficLight
        );
        assert_eq!(
            settings.widget_face_pack,
            crate::core::FacePack::TabyClassic
        );
        assert!(settings.sound_enabled);
        assert_eq!(settings.sound_volume, 75);
        assert!(settings.sound_on_attention);
        assert!(!settings.sound_on_completion);
        assert_eq!(settings.sound_preset, crate::core::SoundPreset::GentleChime);
        assert!(settings.notifications_enabled);
        assert!(!settings.do_not_disturb);
        assert!(settings.do_not_disturb_until.is_none());
        assert!(settings.suppress_when_focused);
    }

    #[test]
    fn do_not_disturb_round_trips_and_defaults_off_for_legacy_settings() {
        let legacy: AppSettings =
            serde_json::from_str(r#"{"notifications_enabled":true}"#).unwrap();
        assert!(!legacy.do_not_disturb);
        assert!(legacy.do_not_disturb_until.is_none());
        assert!(legacy.suppress_when_focused);

        let settings = AppSettings {
            do_not_disturb: false,
            do_not_disturb_until: Some("2026-01-01T12:00:00Z".to_string()),
            suppress_when_focused: false,
            ..AppSettings::default()
        };
        let decoded: AppSettings =
            serde_json::from_slice(&serde_json::to_vec(&settings).unwrap()).unwrap();
        assert_eq!(
            decoded.do_not_disturb_until.as_deref(),
            Some("2026-01-01T12:00:00Z")
        );
        assert!(!decoded.suppress_when_focused);
    }

    #[test]
    fn widget_dimensions_preserve_classic_size_and_rotate_faces_to_landscape() {
        use crate::core::{WidgetSize, WidgetVisualStyle};

        assert_eq!(
            WidgetSize::Standard.dimensions_for(WidgetVisualStyle::TrafficLight),
            (112, 272)
        );
        let (face_width, face_height) =
            WidgetSize::Standard.dimensions_for(WidgetVisualStyle::AnimatedFace);
        assert!(face_width > face_height);
        assert_eq!(face_width, 228);
        assert_eq!(face_height, 140);
    }

    #[test]
    fn legacy_settings_without_agent_visibility_default_to_hidden() {
        let settings: AppSettings =
            serde_json::from_str(r#"{"traffic_light_always_on_top":true}"#).unwrap();
        assert!(settings.agent_widget_visibility.is_empty());
    }

    #[test]
    fn settings_round_trip_through_json() {
        let hotzone = HotZone {
            x: 10,
            y: 20,
            width: 150,
            height: 200,
            monitor_id: Some(0),
        };
        let mut agent_hotzones = HashMap::new();
        agent_hotzones.insert("codex".to_string(), hotzone.clone());
        let mut agent_widget_visibility = HashMap::new();
        agent_widget_visibility.insert("codex".to_string(), true);
        agent_widget_visibility.insert("pi".to_string(), false);

        let settings = AppSettings {
            traffic_light_always_on_top: false,
            traffic_light_position: Some(WidgetPosition { x: 240, y: 180 }),
            hotzone_mode: true,
            agent_hotzones: agent_hotzones,
            agent_widget_visibility,
            default_hotzone: HotZone {
                x: 5,
                y: 5,
                width: 120,
                height: 120,
                monitor_id: None,
            },
            peek_delay_ms: 1500,
            sound_enabled: false,
            sound_volume: 42,
            sound_on_attention: false,
            sound_on_completion: true,
            sound_preset: crate::core::SoundPreset::AlertKnock,
            notifications_enabled: false,
            widget_visual_style: crate::core::WidgetVisualStyle::AnimatedFace,
            widget_face_pack: crate::core::FacePack::TabyExpressive,
            ..AppSettings::default()
        };
        let encoded = serde_json::to_vec(&settings).unwrap();
        let decoded: AppSettings = serde_json::from_slice(&encoded).unwrap();

        assert_eq!(decoded.traffic_light_always_on_top, false);
        assert_eq!(decoded.traffic_light_position.unwrap().x, 240);
        assert!(decoded.hotzone_mode);
        assert_eq!(decoded.agent_hotzones.len(), 1);
        assert!(decoded.agent_hotzones.contains_key("codex"));
        assert_eq!(decoded.agent_widget_visibility.get("codex"), Some(&true));
        assert_eq!(decoded.agent_widget_visibility.get("pi"), Some(&false));
        assert_eq!(decoded.default_hotzone.width, 120);
        assert!(!decoded.sound_enabled);
        assert_eq!(decoded.sound_volume, 42);
        assert!(!decoded.sound_on_attention);
        assert!(decoded.sound_on_completion);
        assert_eq!(decoded.sound_preset, crate::core::SoundPreset::AlertKnock);
        assert!(!decoded.notifications_enabled);
        assert_eq!(
            decoded.widget_visual_style,
            crate::core::WidgetVisualStyle::AnimatedFace
        );
        assert_eq!(
            decoded.widget_face_pack,
            crate::core::FacePack::TabyExpressive
        );
    }

    #[test]
    fn hotzone_contains_point() {
        let hotzone = HotZone {
            x: 100,
            y: 100,
            width: 200,
            height: 200,
            monitor_id: None,
        };

        assert!(hotzone.contains(100, 100));
        assert!(hotzone.contains(299, 299));
        assert!(!hotzone.contains(300, 300));
        assert!(!hotzone.contains(99, 100));
        assert!(!hotzone.contains(100, 99));
    }

    #[test]
    fn hotzone_default_is_bottom_left() {
        let hotzone = HotZone::default_bottom_left();
        assert_eq!(hotzone.x, 0);
        assert_eq!(hotzone.y, 0);
        assert_eq!(hotzone.width, 100);
        assert_eq!(hotzone.height, 100);
        assert!(hotzone.contains(0, 0));
        assert!(hotzone.contains(99, 99));
        assert!(!hotzone.contains(100, 100));
    }

    #[test]
    fn hotzone_preset_positions_are_correct() {
        use crate::core::HotZonePreset;
        let (mw, mh) = (1920u32, 1080u32);
        let size = 100u32;

        assert_eq!(HotZonePreset::TopLeft.position(mw, mh, size), (0, 0));
        assert_eq!(HotZonePreset::TopRight.position(mw, mh, size), (1820, 0));
        assert_eq!(HotZonePreset::BottomLeft.position(mw, mh, size), (0, 980));
        assert_eq!(HotZonePreset::CenterTop.position(mw, mh, size), (910, 0));
        assert_eq!(
            HotZonePreset::CenterBottom.position(mw, mh, size),
            (910, 980)
        );
        assert_eq!(
            HotZonePreset::BottomRight.position(mw, mh, size),
            (1820, 980)
        );
        assert_eq!(HotZonePreset::CenterLeft.position(mw, mh, size), (0, 490));
        assert_eq!(
            HotZonePreset::CenterRight.position(mw, mh, size),
            (1820, 490)
        );
    }

    #[test]
    fn settings_round_trip_includes_peek_delay() {
        let settings = AppSettings {
            peek_delay_ms: 3000,
            ..AppSettings::default()
        };
        let encoded = serde_json::to_vec(&settings).unwrap();
        let decoded: AppSettings = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded.peek_delay_ms, 3000);
    }
}
