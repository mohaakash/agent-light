mod api;
mod commands;
mod core;
mod integrations;
mod outputs;

use commands::audio::{get_audio_settings, set_audio_settings};
use commands::display::{get_display_snapshot, set_manual_display_state};
use commands::focus::{focus_agent_session, get_agent_focus_target};
use commands::hardware::{
    connect_serial_device, disconnect_serial_device, get_hardware_status, list_serial_ports,
    start_hardware_heartbeat, test_hardware_led,
};
use commands::integrations::{
    get_agent_integration_diagnostic, get_agent_integration_status, install_agent_integration,
    restore_agent_integration, send_test_agent_event, uninstall_agent_integration, verify_bridge,
};
use commands::sessions::{get_agent_display_snapshot, get_sessions};
use commands::windows::{
    ensure_agent_widget_window, get_active_hotzone_at, get_agent_hotzone, get_agent_hotzone_at,
    get_agent_hotzone_preset, get_agent_widget_visibility, get_available_monitors,
    get_default_hotzone, get_hotzone_mode, get_hotzone_presets, get_monitor_at, get_peek_delay,
    get_preset_hotzone, get_traffic_light_always_on_top, get_widget_appearance, set_agent_hotzone,
    set_agent_hotzone_preset, set_agent_widget_visibility, set_default_hotzone, set_hotzone_mode,
    set_peek_delay, set_traffic_light_always_on_top, set_traffic_light_visibility,
    set_widget_appearance, start_agent_widget_drag,
};
use core::AppState;
use std::{
    collections::HashMap,
    thread,
    time::{Duration, Instant},
};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
#[cfg(target_os = "windows")]
use tauri::window::Color;
#[cfg(target_os = "macos")]
use tauri::window::EffectState;
#[cfg(any(target_os = "windows", target_os = "macos"))]
use tauri::window::{Effect, EffectsBuilder};
use tauri::{Emitter, Manager, WebviewWindow, WindowEvent};

/// Keep every traffic-light window on a connected display after the display
/// topology changes (unplugged monitor, resolution or scale change).
fn reflow_widget_windows(app: &tauri::AppHandle) {
    let (widget_size, visual_style) = app
        .state::<AppState>()
        .settings
        .lock()
        .map(|settings| (settings.widget_size, settings.widget_visual_style))
        .unwrap_or_default();
    let (width, height) = widget_size.dimensions_for(visual_style);

    for (label, window) in app.webview_windows() {
        if !label.starts_with("traffic-light-") {
            continue;
        }
        let _ = window.set_size(tauri::LogicalSize::new(width, height));
        let Ok(position) = window.outer_position() else {
            continue;
        };
        let Some(clamped) = core::settings::clamp_position(
            &window,
            &core::WidgetPosition {
                x: position.x,
                y: position.y,
            },
            widget_size,
            visual_style,
        ) else {
            continue;
        };
        if clamped.x != position.x || clamped.y != position.y {
            let _ = window.set_position(tauri::PhysicalPosition::new(clamped.x, clamped.y));
        }
    }
}

fn broadcast_hotzone_widget_visibility(
    app: &tauri::AppHandle,
    agent: &str,
    visible: bool,
    anchor: &str,
    hotzone: core::HotZone,
) {
    if let Err(error) = app.emit(
        commands::windows::HOTZONE_WIDGET_EVENT,
        commands::windows::HotzoneWidgetVisibility {
            agent: agent.to_string(),
            visible,
            anchor: anchor.to_string(),
            hotzone,
        },
    ) {
        eprintln!("AgentLight could not broadcast hotzone animation state: {error}");
    }
}

fn anchor_for_hotzone(
    hotzone: &core::HotZone,
    preset: Option<core::HotZonePreset>,
    monitor: &core::MonitorRect,
) -> &'static str {
    if let Some(preset) = preset {
        return match preset {
            core::HotZonePreset::TopLeft => "top_left",
            core::HotZonePreset::TopRight => "top_right",
            core::HotZonePreset::BottomLeft => "bottom_left",
            core::HotZonePreset::BottomRight => "bottom_right",
            core::HotZonePreset::CenterTop => "center_top",
            core::HotZonePreset::CenterBottom => "center_bottom",
            core::HotZonePreset::CenterLeft => "center_left",
            core::HotZonePreset::CenterRight => "center_right",
        };
    }

    let local_x = hotzone.x - monitor.x;
    let local_y = hotzone.y - monitor.y;
    let center_x = local_x + (hotzone.width as i32 / 2);
    let center_y = local_y + (hotzone.height as i32 / 2);
    let horizontal = if (center_x - monitor.width as i32 / 2).abs() < monitor.width as i32 / 6 {
        "center"
    } else if center_x < monitor.width as i32 / 2 {
        "left"
    } else {
        "right"
    };
    let vertical = if center_y < monitor.height as i32 / 3 {
        "top"
    } else if center_y > monitor.height as i32 * 2 / 3 {
        "bottom"
    } else {
        "center"
    };
    match (horizontal, vertical) {
        ("left", "top") => "top_left",
        ("right", "top") => "top_right",
        ("left", "bottom") => "bottom_left",
        ("right", "bottom") => "bottom_right",
        ("center", "top") => "center_top",
        ("center", "bottom") => "center_bottom",
        ("left", _) => "center_left",
        _ => "center_right",
    }
}

fn position_widget_at_hotzone(
    app: &tauri::AppHandle,
    window: &WebviewWindow,
    hotzone: &core::HotZone,
    anchor: &str,
) {
    let monitors = commands::windows::monitor_rects(app);
    let monitor = hotzone
        .monitor_id
        .and_then(|index| monitors.get(index))
        .or_else(|| {
            monitors.iter().find(|monitor| {
                hotzone.x >= monitor.x
                    && hotzone.x < monitor.x.saturating_add(monitor.width as i32)
                    && hotzone.y >= monitor.y
                    && hotzone.y < monitor.y.saturating_add(monitor.height as i32)
            })
        });
    let Some(monitor) = monitor else { return };

    let state = app.state::<AppState>();
    let (size, visual_style) = state
        .settings
        .lock()
        .map(|settings| (settings.widget_size, settings.widget_visual_style))
        .unwrap_or_default();
    let (logical_width, logical_height) = size.dimensions_for(visual_style);
    let scale = if monitor.scale_factor > 0.0 {
        monitor.scale_factor
    } else {
        1.0
    };
    let width = (logical_width as f64 * scale).round() as i32;
    let height = (logical_height as f64 * scale).round() as i32;
    let right = monitor.x + (monitor.width as i32 - width).max(0);
    let bottom = monitor.y + (monitor.height as i32 - height).max(0);
    let center_x = monitor.x + ((monitor.width as i32 - width) / 2).max(0);
    let center_y = monitor.y + ((monitor.height as i32 - height) / 2).max(0);
    let (x, y) = match anchor {
        "top_left" => (monitor.x, monitor.y),
        "top_right" => (right, monitor.y),
        "bottom_left" => (monitor.x, bottom),
        "bottom_right" => (right, bottom),
        "center_top" => (center_x, monitor.y),
        "center_bottom" => (center_x, bottom),
        "center_left" => (monitor.x, center_y),
        "center_right" => (right, center_y),
        _ => (monitor.x, monitor.y),
    };
    let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
}

fn start_hotzone_monitor(app: tauri::AppHandle) {
    const AGENTS: [&str; 8] = [
        "codex",
        "claude",
        "cursor",
        "gemini",
        "windsurf",
        "copilot",
        "antigravity",
        "pi",
    ];

    thread::spawn(move || {
        const EXIT_ANIMATION: Duration = Duration::from_millis(700);
        let mut outside_since: HashMap<&str, Instant> = HashMap::new();
        let mut hide_animation_started: HashMap<&str, Instant> = HashMap::new();
        let mut monitors = commands::windows::monitor_rects(&app);
        let mut topology_checked_at = Instant::now();

        loop {
            // Detect display topology changes by comparing the live monitor
            // list; there is no cross-platform OS notification for this.
            if topology_checked_at.elapsed() >= Duration::from_secs(2) {
                topology_checked_at = Instant::now();
                let current = commands::windows::monitor_rects(&app);
                if current != monitors {
                    monitors = current;
                    reflow_widget_windows(&app);
                    if let Err(error) =
                        app.emit(commands::windows::DISPLAY_TOPOLOGY_EVENT, monitors.clone())
                    {
                        eprintln!("AgentLight could not broadcast a display change: {error}");
                    }
                }
            }

            let app_state = app.state::<AppState>();
            let (mode_enabled, peek_delay_ms, hotzones, hotzone_presets, enabled_agents) =
                match app_state.settings.lock() {
                    Ok(settings) => (
                        settings.hotzone_mode,
                        settings.peek_delay_ms,
                        settings.agent_hotzones.clone(),
                        settings.agent_hotzone_presets.clone(),
                        settings.agent_widget_visibility.clone(),
                    ),
                    Err(_) => {
                        thread::sleep(Duration::from_millis(100));
                        continue;
                    }
                };

            if !mode_enabled {
                outside_since.clear();
                hide_animation_started.clear();
                thread::sleep(Duration::from_millis(50));
                continue;
            }

            let cursor = app
                .cursor_position()
                .ok()
                .map(|position| (position.x.round() as i32, position.y.round() as i32));
            let now = Instant::now();
            let hide_delay = Duration::from_millis(peek_delay_ms as u64);

            for agent in AGENTS {
                let label = format!("traffic-light-{agent}");
                let Some(window) = app.get_webview_window(&label) else {
                    continue;
                };
                let is_enabled = enabled_agents.get(agent).copied().unwrap_or(false);
                let preset_definition = hotzone_presets.get(agent);
                let hotzone = preset_definition
                    .and_then(|preset| preset.resolve(&monitors))
                    .or_else(|| hotzones.get(agent).cloned());
                let anchor = hotzone.as_ref().and_then(|hotzone| {
                    let monitor = hotzone
                        .monitor_id
                        .and_then(|index| monitors.get(index))
                        .or_else(|| {
                            monitors.iter().find(|monitor| {
                                hotzone.x >= monitor.x
                                    && hotzone.x < monitor.x.saturating_add(monitor.width as i32)
                                    && hotzone.y >= monitor.y
                                    && hotzone.y < monitor.y.saturating_add(monitor.height as i32)
                            })
                        })?;
                    Some(anchor_for_hotzone(
                        hotzone,
                        preset_definition.map(|preset| preset.preset),
                        monitor,
                    ))
                });
                let is_inside = is_enabled
                    && cursor.is_some_and(|(x, y)| {
                        hotzone
                            .as_ref()
                            .is_some_and(|hotzone| hotzone.contains(x, y))
                    });
                let is_visible = window.is_visible().unwrap_or(false);

                if is_inside {
                    outside_since.remove(agent);
                    let reversing_exit = hide_animation_started.remove(agent).is_some();
                    if !is_visible {
                        if let (Some(hotzone), Some(anchor)) = (hotzone.as_ref(), anchor) {
                            position_widget_at_hotzone(&app, &window, hotzone, anchor);
                        }
                        let _ = window.set_always_on_top(true);
                        if let Err(error) = window.show() {
                            eprintln!("Unable to reveal hotzone widget {label}: {error}");
                        } else if let (Some(hotzone), Some(anchor)) = (hotzone.clone(), anchor) {
                            broadcast_hotzone_widget_visibility(&app, agent, true, anchor, hotzone);
                        }
                    } else if reversing_exit {
                        if let (Some(hotzone), Some(anchor)) = (hotzone.clone(), anchor) {
                            broadcast_hotzone_widget_visibility(&app, agent, true, anchor, hotzone);
                        }
                    }
                } else if is_visible {
                    if let Some(hiding_at) = hide_animation_started.get(agent).copied() {
                        if now.duration_since(hiding_at) >= EXIT_ANIMATION {
                            if let Err(error) = window.hide() {
                                eprintln!("Unable to hide hotzone widget {label}: {error}");
                            }
                            hide_animation_started.remove(agent);
                            outside_since.remove(agent);
                        }
                    } else {
                        let left_at = outside_since.entry(agent).or_insert(now);
                        if now.duration_since(*left_at) >= hide_delay {
                            if let (Some(hotzone), Some(anchor)) = (hotzone.clone(), anchor) {
                                broadcast_hotzone_widget_visibility(
                                    &app, agent, false, anchor, hotzone,
                                );
                            }
                            hide_animation_started.insert(agent, now);
                        }
                    }
                } else {
                    outside_since.remove(agent);
                    hide_animation_started.remove(agent);
                }
            }

            thread::sleep(Duration::from_millis(50));
        }
    });
}

fn log_startup_stage(started_at: Instant, stage: &str) {
    #[cfg(debug_assertions)]
    eprintln!(
        "[startup] native {stage}: +{}ms",
        started_at.elapsed().as_millis()
    );
    #[cfg(not(debug_assertions))]
    let _ = (started_at, stage);
}

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
        .plugin(tauri_plugin_notification::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            get_display_snapshot,
            set_manual_display_state,
            set_traffic_light_visibility,
            set_traffic_light_always_on_top,
            get_traffic_light_always_on_top,
            get_audio_settings,
            set_audio_settings,
            get_widget_appearance,
            set_widget_appearance,
            list_serial_ports,
            get_hardware_status,
            connect_serial_device,
            disconnect_serial_device,
            test_hardware_led,
            install_agent_integration,
            uninstall_agent_integration,
            get_agent_integration_status,
            get_agent_integration_diagnostic,
            restore_agent_integration,
            verify_bridge,
            send_test_agent_event,
            get_sessions,
            get_agent_focus_target,
            focus_agent_session,
            get_agent_display_snapshot,
            get_agent_widget_visibility,
            set_agent_widget_visibility,
            start_agent_widget_drag,
            get_hotzone_mode,
            set_hotzone_mode,
            get_agent_hotzone,
            get_agent_hotzone_preset,
            set_agent_hotzone,
            set_agent_hotzone_preset,
            get_active_hotzone_at,
            get_default_hotzone,
            set_default_hotzone,
            get_agent_hotzone_at,
            get_available_monitors,
            get_monitor_at,
            get_peek_delay,
            set_peek_delay,
            get_hotzone_presets,
            get_preset_hotzone
        ])
        .on_window_event(|window, event| {
            if window.label() == "main" {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
                return;
            }
            if !window.label().starts_with("traffic-light-") {
                return;
            }

            let app = window.app_handle();
            let state = app.state::<AppState>();

            match event {
                WindowEvent::Moved(_) => {
                    let Ok(position) = window.outer_position() else {
                        return;
                    };
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
                }
                // Mixed-DPI moves hand the window to a monitor with a different
                // scale factor. Re-apply the logical size and re-clamp so the
                // widget does not jump or grow beyond the new screen bounds.
                WindowEvent::ScaleFactorChanged { .. } => {
                    let (widget_size, visual_style) = state
                        .settings
                        .lock()
                        .map(|settings| (settings.widget_size, settings.widget_visual_style))
                        .unwrap_or_default();
                    let (width, height) = widget_size.dimensions_for(visual_style);
                    let _ = window.set_size(tauri::LogicalSize::new(width, height));
                    let Some(webview_window) = app.get_webview_window(window.label()) else {
                        return;
                    };
                    if let Ok(position) = window.outer_position() {
                        if let Some(clamped) = core::settings::clamp_position(
                            &webview_window,
                            &core::WidgetPosition {
                                x: position.x,
                                y: position.y,
                            },
                            widget_size,
                            visual_style,
                        ) {
                            let _ = window
                                .set_position(tauri::PhysicalPosition::new(clamped.x, clamped.y));
                        }
                    }
                }
                _ => {}
            }
        })
        .setup(|app| {
            let startup_started_at = Instant::now();
            log_startup_stage(startup_started_at, "setup entered");
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
            log_startup_stage(startup_started_at, "settings loaded");

            let bridge = integrations::codex::initialize_bridge(app.handle())
                .map_err(std::io::Error::other)?;
            log_startup_stage(startup_started_at, "bridge initialized");

            if let Err(error) = api::local_server::start(app.handle().clone(), bridge.token) {
                eprintln!("AgentLight Codex bridge unavailable: {error}");
            }
            integrations::codex::start_session_maintenance(app.handle().clone());
            start_hardware_heartbeat(app.handle().clone());
            log_startup_stage(startup_started_at, "background services started");

            if let Some(window) = app.get_webview_window("main") {
                let glass_started_at = Instant::now();
                apply_glass_effect(&window)?;
                log_startup_stage(glass_started_at, "glass effect duration");
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
            let toggle_sound = MenuItem::with_id(
                app,
                "toggle-audio",
                "Toggle sound alerts",
                true,
                None::<&str>,
            )?;
            let toggle_dnd = MenuItem::with_id(
                app,
                "toggle-dnd",
                "Toggle do not disturb",
                true,
                None::<&str>,
            )?;
            let separator = PredefinedMenuItem::separator(app)?;
            let quit = MenuItem::with_id(app, "quit", "Quit AgentLight", true, None::<&str>)?;
            let menu = Menu::with_items(
                app,
                &[
                    &show_dashboard,
                    &toggle_widget,
                    &toggle_sound,
                    &toggle_dnd,
                    &separator,
                    &quit,
                ],
            )?;
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
                    "toggle-audio" => {
                        if let Err(error) = commands::audio::toggle_sound_enabled(app.clone()) {
                            eprintln!("AgentLight could not toggle sound alerts: {error}");
                        }
                    }
                    "toggle-dnd" => {
                        if let Err(error) = commands::audio::toggle_do_not_disturb(app.clone()) {
                            eprintln!("AgentLight could not toggle do not disturb: {error}");
                        }
                    }
                    "toggle-widget" => {
                        let (enabled, hotzone_mode) = app
                            .state::<AppState>()
                            .settings
                            .lock()
                            .map(|settings| {
                                (
                                    settings
                                        .agent_widget_visibility
                                        .get("codex")
                                        .copied()
                                        .unwrap_or(false),
                                    settings.hotzone_mode,
                                )
                            })
                            .unwrap_or((false, false));
                        if !enabled || hotzone_mode {
                            if let Some(window) = app.get_webview_window("traffic-light-codex") {
                                let _ = window.hide();
                            }
                        } else if let Some(window) = app.get_webview_window("traffic-light-codex") {
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                            }
                        } else if let Ok(window) = ensure_agent_widget_window(app, "codex") {
                            let _ = window.show();
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
            log_startup_stage(startup_started_at, "tray menu created");

            for agent in [
                "codex",
                "claude",
                "cursor",
                "gemini",
                "windsurf",
                "copilot",
                "antigravity",
                "pi",
            ] {
                if loaded_settings
                    .agent_widget_visibility
                    .get(agent)
                    .copied()
                    .unwrap_or(false)
                {
                    let window = ensure_agent_widget_window(app.handle(), agent)
                        .map_err(std::io::Error::other)?;
                    if !loaded_settings.hotzone_mode {
                        window.show()?;
                    }
                }
            }

            log_startup_stage(startup_started_at, "enabled widgets created");
            start_hotzone_monitor(app.handle().clone());
            log_startup_stage(startup_started_at, "setup complete");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running AgentLight");
}
