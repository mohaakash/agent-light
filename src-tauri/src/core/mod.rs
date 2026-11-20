pub mod display_state;
pub mod session;
pub mod settings;

pub use display_state::{DisplaySnapshot, DisplayState};

use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Mutex};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct WidgetPosition {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct HotZone {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub monitor_id: Option<usize>,
}

/// Live geometry of a display, mirroring the values Tauri reports.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub struct MonitorRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
}

/// Resolution-independent hotzone definition: a screen corner plus a logical
/// size, resolved against the current display topology on every hit test.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct NormalizedHotZone {
    pub monitor_index: usize,
    pub preset: HotZonePreset,
    pub size_logical: u32,
    #[serde(default)]
    pub offset_logical: (i32, i32),
}

impl NormalizedHotZone {
    /// Convert this logical definition into a physical screen rectangle using
    /// the monitor's live position, size, and scale factor.
    pub fn resolve(&self, monitors: &[MonitorRect]) -> Option<HotZone> {
        let monitor = monitors.get(self.monitor_index)?;
        let scale = if monitor.scale_factor > 0.0 {
            monitor.scale_factor
        } else {
            1.0
        };
        let physical_size = ((self.size_logical as f64) * scale).round().max(8.0) as u32;
        let physical_size = physical_size
            .min(monitor.width.max(8))
            .min(monitor.height.max(8));
        let (base_x, base_y) = self
            .preset
            .position(monitor.width, monitor.height, physical_size);
        let offset_x = (self.offset_logical.0 as f64 * scale).round() as i32;
        let offset_y = (self.offset_logical.1 as f64 * scale).round() as i32;
        let max_x = monitor.width.saturating_sub(physical_size) as i32;
        let max_y = monitor.height.saturating_sub(physical_size) as i32;
        let x = (base_x + offset_x).clamp(0, max_x.max(0));
        let y = (base_y + offset_y).clamp(0, max_y.max(0));

        Some(HotZone {
            x: monitor.x + x,
            y: monitor.y + y,
            width: physical_size,
            height: physical_size,
            monitor_id: Some(self.monitor_index),
        })
    }
}

impl HotZone {
    /// Create a default hotzone in the bottom-left corner (GNOME-style)
    pub fn default_bottom_left() -> Self {
        Self {
            x: 0,
            y: 0,
            width: 100,
            height: 100,
            monitor_id: None,
        }
    }

    /// Check if a point (x, y) is within this hotzone
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x
            && x < (self.x + self.width as i32)
            && y >= self.y
            && y < (self.y + self.height as i32)
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WidgetSize {
    Compact,
    #[default]
    Standard,
    Large,
}

impl WidgetSize {
    /// Existing classic traffic-light window sizes. Keep these unchanged.
    pub fn dimensions(self) -> (u32, u32) {
        match self {
            Self::Compact => (90, 218),
            Self::Standard => (112, 272),
            Self::Large => (134, 326),
        }
    }

    /// Taby source GIFs are 280x456 portrait, but their intended desktop/device
    /// presentation is rotated into a landscape face viewport.
    pub fn dimensions_for(self, style: WidgetVisualStyle) -> (u32, u32) {
        if style == WidgetVisualStyle::TrafficLight {
            return self.dimensions();
        }
        match self {
            Self::Compact => (182, 112),
            Self::Standard => (228, 140),
            Self::Large => (274, 168),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WidgetTheme {
    #[default]
    Dark,
    Light,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WidgetVisualStyle {
    #[default]
    TrafficLight,
    AnimatedFace,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FacePack {
    #[default]
    TabyClassic,
    TabyCalm,
    TabyExpressive,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct WidgetAppearance {
    pub size: WidgetSize,
    pub theme: WidgetTheme,
    pub opacity: u8,
    pub visual_style: WidgetVisualStyle,
    pub face_pack: FacePack,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SoundPreset {
    #[default]
    GentleChime,
    AlertKnock,
    ModernPing,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct AudioAlertSettings {
    pub sound_enabled: bool,
    pub sound_volume: u8,
    pub sound_on_attention: bool,
    pub sound_on_completion: bool,
    pub sound_preset: SoundPreset,
    pub notifications_enabled: bool,
    /// Silences every alert (sounds and notifications) until cleared.
    pub do_not_disturb: bool,
    /// Optional end of a timed do-not-disturb window, as an RFC 3339 timestamp.
    pub do_not_disturb_until: Option<String>,
    /// Suppress alerts while the AgentLight dashboard is focused.
    pub suppress_when_focused: bool,
}

impl Default for AudioAlertSettings {
    fn default() -> Self {
        Self {
            sound_enabled: true,
            sound_volume: 75,
            sound_on_attention: true,
            sound_on_completion: false,
            sound_preset: SoundPreset::GentleChime,
            notifications_enabled: true,
            do_not_disturb: false,
            do_not_disturb_until: None,
            suppress_when_focused: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HotZonePreset {
    TopLeft,
    TopRight,
    #[default]
    BottomLeft,
    BottomRight,
    CenterTop,
    CenterBottom,
    CenterLeft,
    CenterRight,
}

impl HotZonePreset {
    /// Get corner position for a preset relative to a monitor of given dimensions
    pub fn position(self, monitor_width: u32, monitor_height: u32, size: u32) -> (i32, i32) {
        let s = size as i32;
        let mw = monitor_width as i32;
        let mh = monitor_height as i32;
        match self {
            Self::TopLeft => (0, 0),
            Self::TopRight => (mw.saturating_sub(s), 0),
            Self::BottomLeft => (0, mh.saturating_sub(s)),
            Self::BottomRight => (mw.saturating_sub(s), mh.saturating_sub(s)),
            Self::CenterTop => (mw.saturating_sub(s) / 2, 0),
            Self::CenterBottom => (mw.saturating_sub(s) / 2, mh.saturating_sub(s)),
            Self::CenterLeft => (0, (mh - s) / 2),
            Self::CenterRight => (mw.saturating_sub(s), (mh - s) / 2),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct AppSettings {
    pub traffic_light_always_on_top: bool,
    pub traffic_light_position: Option<WidgetPosition>,
    #[serde(default)]
    pub agent_widget_positions: HashMap<String, WidgetPosition>,
    #[serde(default)]
    pub agent_widget_visibility: HashMap<String, bool>,
    pub hotzone_mode: bool,
    #[serde(default)]
    pub agent_hotzones: HashMap<String, HotZone>,
    /// Normalized hotzone definitions. When present they take precedence over
    /// the absolute `agent_hotzones` cache so hotzones follow display changes.
    #[serde(default)]
    pub agent_hotzone_presets: HashMap<String, NormalizedHotZone>,
    pub default_hotzone: HotZone,
    pub widget_size: WidgetSize,
    pub widget_theme: WidgetTheme,
    pub widget_opacity: u8,
    #[serde(default)]
    pub widget_visual_style: WidgetVisualStyle,
    #[serde(default)]
    pub widget_face_pack: FacePack,
    pub peek_delay_ms: u32,
    pub sound_enabled: bool,
    pub sound_volume: u8,
    pub sound_on_attention: bool,
    pub sound_on_completion: bool,
    pub sound_preset: SoundPreset,
    pub notifications_enabled: bool,
    #[serde(default)]
    pub do_not_disturb: bool,
    #[serde(default)]
    pub do_not_disturb_until: Option<String>,
    #[serde(default = "default_suppress_when_focused")]
    pub suppress_when_focused: bool,
}

fn default_suppress_when_focused() -> bool {
    true
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            traffic_light_always_on_top: true,
            traffic_light_position: None,
            agent_widget_positions: HashMap::new(),
            agent_widget_visibility: HashMap::new(),
            hotzone_mode: false,
            agent_hotzones: HashMap::new(),
            agent_hotzone_presets: HashMap::new(),
            default_hotzone: HotZone::default_bottom_left(),
            widget_size: WidgetSize::Standard,
            widget_theme: WidgetTheme::Dark,
            widget_opacity: 100,
            widget_visual_style: WidgetVisualStyle::TrafficLight,
            widget_face_pack: FacePack::TabyClassic,
            peek_delay_ms: 2000,
            sound_enabled: true,
            sound_volume: 75,
            sound_on_attention: true,
            sound_on_completion: false,
            sound_preset: SoundPreset::GentleChime,
            notifications_enabled: true,
            do_not_disturb: false,
            do_not_disturb_until: None,
            suppress_when_focused: true,
        }
    }
}

pub struct AppState {
    pub display: Mutex<DisplaySnapshot>,
    pub settings: Mutex<AppSettings>,
    pub sessions: Mutex<session::SessionManager>,
    pub hardware: crate::outputs::hardware::HardwareOutput,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            display: Mutex::new(DisplaySnapshot::manual(DisplayState::Idle)),
            settings: Mutex::new(AppSettings::default()),
            sessions: Mutex::new(session::SessionManager::default()),
            hardware: crate::outputs::hardware::HardwareOutput::default(),
        }
    }
}

#[cfg(test)]
mod hotzone_geometry_tests {
    use super::{HotZonePreset, MonitorRect, NormalizedHotZone};

    fn monitor(x: i32, y: i32, width: u32, height: u32, scale_factor: f64) -> MonitorRect {
        MonitorRect {
            x,
            y,
            width,
            height,
            scale_factor,
        }
    }

    #[test]
    fn normalized_hotzone_scales_with_dpi() {
        let monitors = [
            monitor(0, 0, 3840, 2160, 1.5),
            monitor(-1920, 0, 1920, 1080, 1.0),
        ];

        let hz = NormalizedHotZone {
            monitor_index: 0,
            preset: HotZonePreset::TopRight,
            size_logical: 120,
            offset_logical: (0, 0),
        }
        .resolve(&monitors)
        .unwrap();

        assert_eq!(hz.width, 180, "120 logical px at 150% is 180 physical px");
        assert_eq!(hz.height, 180);
        assert_eq!(hz.x, 3840 - 180);
        assert_eq!(hz.y, 0);
        assert_eq!(hz.monitor_id, Some(0));
    }

    #[test]
    fn normalized_hotzone_handles_negative_monitor_origins() {
        let monitors = [
            monitor(0, 0, 1920, 1080, 1.0),
            monitor(-1920, 0, 1920, 1080, 1.0),
        ];

        let hz = NormalizedHotZone {
            monitor_index: 1,
            preset: HotZonePreset::BottomRight,
            size_logical: 100,
            offset_logical: (0, 0),
        }
        .resolve(&monitors)
        .unwrap();

        assert_eq!(hz.x, -100);
        assert_eq!(hz.y, 980);
        assert!(
            hz.contains(-50, 1000),
            "hit tests must work in negative space"
        );
        assert!(!hz.contains(10, 1000));
    }

    #[test]
    fn normalized_hotzone_offset_is_applied_in_logical_pixels() {
        let monitors = [monitor(0, 0, 2560, 1440, 2.0)];

        let hz = NormalizedHotZone {
            monitor_index: 0,
            preset: HotZonePreset::TopLeft,
            size_logical: 100,
            offset_logical: (20, 30),
        }
        .resolve(&monitors)
        .unwrap();

        assert_eq!(hz.width, 200);
        assert_eq!(hz.x, 40, "20 logical px at 200% is 40 physical px");
        assert_eq!(hz.y, 60);
    }

    #[test]
    fn normalized_hotzone_clamps_offsets_inside_the_monitor() {
        let monitors = [monitor(0, 0, 1920, 1080, 1.0)];

        let hz = NormalizedHotZone {
            monitor_index: 0,
            preset: HotZonePreset::BottomRight,
            size_logical: 100,
            offset_logical: (5000, 5000),
        }
        .resolve(&monitors)
        .unwrap();

        assert_eq!(hz.x, 1820);
        assert_eq!(hz.y, 980);
    }

    #[test]
    fn normalized_hotzone_returns_none_for_a_missing_monitor() {
        let monitors = [monitor(0, 0, 1920, 1080, 1.0)];
        let hz = NormalizedHotZone {
            monitor_index: 3,
            preset: HotZonePreset::TopLeft,
            size_logical: 100,
            offset_logical: (0, 0),
        };
        assert!(hz.resolve(&monitors).is_none());
    }
}
