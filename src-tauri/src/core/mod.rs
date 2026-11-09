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

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WidgetSize {
    Compact,
    #[default]
    Standard,
    Large,
}

impl WidgetSize {
    pub fn dimensions(self) -> (u32, u32) {
        match self {
            Self::Compact => (90, 218),
            Self::Standard => (112, 272),
            Self::Large => (134, 326),
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

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct WidgetAppearance {
    pub size: WidgetSize,
    pub theme: WidgetTheme,
    pub opacity: u8,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct AppSettings {
    pub traffic_light_always_on_top: bool,
    pub traffic_light_position: Option<WidgetPosition>,
    #[serde(default)]
    pub agent_widget_positions: HashMap<String, WidgetPosition>,
    pub widget_size: WidgetSize,
    pub widget_theme: WidgetTheme,
    pub widget_opacity: u8,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            traffic_light_always_on_top: true,
            traffic_light_position: None,
            agent_widget_positions: HashMap::new(),
            widget_size: WidgetSize::Standard,
            widget_theme: WidgetTheme::Dark,
            widget_opacity: 100,
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
