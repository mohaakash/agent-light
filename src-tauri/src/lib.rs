mod api;
mod commands;
mod core;
mod integrations;
mod outputs;

use commands::display::{get_display_snapshot, set_manual_display_state};
use commands::hardware::{
    connect_serial_device, disconnect_serial_device, get_hardware_status, list_serial_ports,
    start_hardware_heartbeat, test_hardware_led,
};
use commands::sessions::{get_agent_display_snapshot, get_sessions};
use commands::windows::{
    get_traffic_light_always_on_top, get_widget_appearance, set_agent_widget_visibility,
    set_traffic_light_always_on_top, set_traffic_light_visibility, set_widget_appearance,
    start_agent_widget_drag,
};
use core::AppState;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
#[cfg(target_os = "windows")]
use tauri::window::Color;
#[cfg(target_os = "macos")]
use tauri::window::EffectState;
#[cfg(any(target_os = "windows", target_os = "macos"))]
use tauri::window::{Effect, EffectsBuilder};
use tauri::{Manager, WebviewWindow, WindowEvent};

fn apply_glass_effect(window: &WebviewWindow) -> tauri::Result<()> {
    #[cfg(target_os = "windows")]
    window.set_effects(
        EffectsBuilder::new()
            .effect(Effect::Acrylic)
            .color(Color(18, 22, 34, 172))
            .build(),
    )?;

    #[cfg(target_os = "macos")]
    window.set_effects(
        EffectsBuilder::new()
            .effect(Effect::LiquidGlassRegular)
            .effect(Effect::Sidebar)
            .state(EffectState::Active)
            .radius(18.0)
            .build(),
    )?;

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let _ = window;

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            get_display_snapshot,
            set_manual_display_state,
            set_traffic_light_visibility,
            set_traffic_light_always_on_top,
            get_traffic_light_always_on_top,
            get_widget_appearance,
            set_widget_appearance,
            list_serial_ports,
            get_hardware_status,
            connect_serial_device,
            disconnect_serial_device,
            test_hardware_led,
            get_sessions,
            get_agent_display_snapshot,
            set_agent_widget_visibility,
            start_agent_widget_drag
        ])
        .on_window_event(|window, event| {
            if window.label() == "main" {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
                return;
            }
            if !window.label().starts_with("traffic-light-")
                || !matches!(event, WindowEvent::Moved(_))
            {
                return;
            }

            let Ok(position) = window.outer_position() else {
                return;
            };
            let app = window.app_handle();
            let state = app.state::<AppState>();
            if let Ok(mut settings) = state.settings.lock() {
                let Some(agent) = window.label().strip_prefix("traffic-light-") else {
                    return;
                };
                settings.agent_widget_positions.insert(
                    agent.to_string(),
                    core::WidgetPosition {
                        x: position.x,
                        y: position.y,
                    },
                );
                if let Err(error) = core::settings::save(&app, &settings) {
                    eprintln!("AgentLight settings could not be saved: {error}");
                }
            };
        })
        .setup(|app| {
            let loaded_settings = match core::settings::load(app.handle()) {
                Ok(settings) => settings,
                Err(error) => {
                    eprintln!("AgentLight settings could not be loaded: {error}");
                    core::AppSettings::default()
                }
            };
            if let Ok(mut settings) = app.state::<AppState>().settings.lock() {
                *settings = loaded_settings.clone();
            }

            let bridge = integrations::codex::initialize_bridge(app.handle())
                .map_err(std::io::Error::other)?;

            if let Err(error) = api::local_server::start(app.handle().clone(), bridge.token) {
                eprintln!("AgentLight Codex bridge unavailable: {error}");
            }
            integrations::codex::start_session_maintenance(app.handle().clone());
            start_hardware_heartbeat(app.handle().clone());

            if let Some(window) = app.get_webview_window("main") {
                apply_glass_effect(&window)?;
            }

            let show_dashboard =
                MenuItem::with_id(app, "show-dashboard", "Show AgentLight", true, None::<&str>)?;
            let toggle_widget = MenuItem::with_id(
                app,
                "toggle-widget",
                "Show or hide traffic light",
                true,
                None::<&str>,
            )?;
            let separator = PredefinedMenuItem::separator(app)?;
            let quit = MenuItem::with_id(app, "quit", "Quit AgentLight", true, None::<&str>)?;
            let menu =
                Menu::with_items(app, &[&show_dashboard, &toggle_widget, &separator, &quit])?;
            let icon = app
                .default_window_icon()
                .cloned()
                .ok_or_else(|| std::io::Error::other("default app icon is unavailable"))?;
            TrayIconBuilder::with_id("agentlight-tray")
                .icon(icon)
                .tooltip("AgentLight")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "show-dashboard" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "toggle-widget" => {
                        if let Some(window) = app.get_webview_window("traffic-light-codex") {
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                            }
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        if let Some(window) = tray.app_handle().get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            for label in [
                "traffic-light-codex",
                "traffic-light-claude",
                "traffic-light-pi",
            ] {
                if let Some(window) = app.get_webview_window(label) {
                    window.set_always_on_top(loaded_settings.traffic_light_always_on_top)?;
                    let (width, height) = loaded_settings.widget_size.dimensions();
                    window.set_size(tauri::LogicalSize::new(width, height))?;
                    if let Some(position) = loaded_settings
                        .agent_widget_positions
                        .get(label.strip_prefix("traffic-light-").unwrap_or(label))
                        .or(loaded_settings.traffic_light_position.as_ref())
                        .and_then(|position| {
                            core::settings::clamp_position(
                                &window,
                                position,
                                loaded_settings.widget_size,
                            )
                        })
                    {
                        window
                            .set_position(tauri::PhysicalPosition::new(position.x, position.y))?;
                    }
                    window.hide()?;
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running AgentLight");
}
