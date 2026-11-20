import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { DisplaySnapshot, DisplayState } from "./display";

export const DISPLAY_STATE_EVENT = "display-state-changed";
export const HARDWARE_STATUS_EVENT = "hardware-status-changed";
export const WIDGET_APPEARANCE_EVENT = "widget-appearance-changed";
export const AUDIO_SETTINGS_EVENT = "audio-settings-changed";
export const AGENT_ALERT_EVENT = "agent-alert";

export type SoundPreset = "gentle_chime" | "alert_knock" | "modern_ping";
export type AgentAlertKind = "attention" | "completion";

export interface AgentAlertEvent {
  kind: AgentAlertKind;
  agent: string;
  project: string | null;
  state: DisplayState;
}

export function subscribeToAgentAlerts(handler: (alert: AgentAlertEvent) => void): Promise<UnlistenFn> {
  return listen<AgentAlertEvent>(AGENT_ALERT_EVENT, (event) => handler(event.payload));
}

export interface AudioAlertSettings {
  sound_enabled: boolean;
  sound_volume: number;
  sound_on_attention: boolean;
  sound_on_completion: boolean;
  sound_preset: SoundPreset;
  notifications_enabled: boolean;
  do_not_disturb: boolean;
  do_not_disturb_until: string | null;
  suppress_when_focused: boolean;
}

export function getAudioSettings(): Promise<AudioAlertSettings> {
  return invoke<AudioAlertSettings>("get_audio_settings");
}

export function setAudioSettings(settings: AudioAlertSettings): Promise<AudioAlertSettings> {
  return invoke<AudioAlertSettings>("set_audio_settings", { settings });
}

export function subscribeToAudioSettings(handler: (settings: AudioAlertSettings) => void): Promise<UnlistenFn> {
  return listen<AudioAlertSettings>(AUDIO_SETTINGS_EVENT, (event) => handler(event.payload));
}

export type WidgetSize = "compact" | "standard" | "large";
export type WidgetTheme = "dark" | "light";
export type WidgetVisualStyle = "traffic_light" | "animated_face";
export type FacePack = "taby_classic" | "taby_calm" | "taby_expressive";

export interface WidgetAppearance {
  size: WidgetSize;
  theme: WidgetTheme;
  opacity: number;
  visual_style: WidgetVisualStyle;
  face_pack: FacePack;
}

export function getWidgetAppearance(): Promise<WidgetAppearance> {
  return invoke<WidgetAppearance>("get_widget_appearance");
}

export function setWidgetAppearance(appearance: WidgetAppearance): Promise<WidgetAppearance> {
  return invoke<WidgetAppearance>("set_widget_appearance", {
    size: appearance.size,
    theme: appearance.theme,
    opacity: appearance.opacity,
    visualStyle: appearance.visual_style,
    facePack: appearance.face_pack,
  });
}

export function subscribeToWidgetAppearance(handler: (appearance: WidgetAppearance) => void): Promise<UnlistenFn> {
  return listen<WidgetAppearance>(WIDGET_APPEARANCE_EVENT, (event) => handler(event.payload));
}

export interface SerialPortOption {
  port_name: string;
  description: string | null;
}

export interface HardwareStatus {
  connected: boolean;
  port_name: string | null;
  device_id: string | null;
  error: string | null;
}

export interface AgentSession {
  agent: string;
  session_id: string;
  project: string | null;
  state: DisplayState;
  seconds_since_event: number;
  cwd: string | null;
  client_app: string | null;
  process_id: number | null;
}

export interface FocusTarget {
  agent: string;
  session_id: string;
  project: string | null;
  cwd: string | null;
  client_app: string | null;
  process_id: number | null;
  state: DisplayState;
}

export interface FocusOutcome {
  activated: boolean;
  method: string;
  detail: string;
}

export function getAgentFocusTarget(agent: string): Promise<FocusTarget | null> {
  return invoke<FocusTarget | null>("get_agent_focus_target", { agent });
}

export function focusAgentSession(agent: string): Promise<FocusOutcome> {
  return invoke<FocusOutcome>("focus_agent_session", { agent });
}

export function getSessions(): Promise<AgentSession[]> {
  return invoke<AgentSession[]>("get_sessions");
}

export interface IntegrationDiagnostic {
  agent: string;
  installed: boolean;
  config_path: string;
  config_exists: boolean;
  bridge_valid: boolean;
  status_text: string;
  backup_available: boolean;
  backup_path: string | null;
  hook_command: string | null;
}

export interface BridgeVerification {
  reachable: boolean;
  token_matches: boolean;
  endpoint: string;
  detail: string;
}

export function installAgentIntegration(agent: "codex" | "claude" | "cursor" | "gemini" | "windsurf" | "copilot" | "antigravity"): Promise<string> {
  return invoke<string>("install_agent_integration", { agent });
}

export function uninstallAgentIntegration(agent: "codex" | "claude" | "cursor" | "gemini" | "windsurf" | "copilot" | "antigravity"): Promise<string> {
  return invoke<string>("uninstall_agent_integration", { agent });
}

export function restoreAgentIntegration(agent: "codex" | "claude" | "cursor" | "gemini" | "windsurf" | "copilot" | "antigravity"): Promise<string> {
  return invoke<string>("restore_agent_integration", { agent });
}

export function verifyBridge(): Promise<BridgeVerification> {
  return invoke<BridgeVerification>("verify_bridge");
}

export function getAgentIntegrationDiagnostic(agent: "codex" | "claude" | "cursor" | "gemini" | "windsurf" | "copilot" | "antigravity"): Promise<IntegrationDiagnostic> {
  return invoke<IntegrationDiagnostic>("get_agent_integration_diagnostic", { agent });
}

export function sendTestAgentEvent(agent: "codex" | "claude" | "cursor" | "gemini" | "windsurf" | "copilot" | "antigravity", eventType: "permission_required" | "agent_working" | "task_completed" | "session_idle"): Promise<void> {
  return invoke("send_test_agent_event", { agent, event_type: eventType });
}

export function getAgentDisplaySnapshot(agent: string): Promise<DisplaySnapshot> {
  return invoke<DisplaySnapshot>("get_agent_display_snapshot", { agent });
}

export function getAgentWidgetVisibility(agent: string): Promise<boolean> {
  return invoke<boolean>("get_agent_widget_visibility", { agent });
}

export function setAgentWidgetVisibility(agent: string, visible: boolean): Promise<void> {
  return invoke("set_agent_widget_visibility", { agent, visible });
}

export function startAgentWidgetDrag(agent: string): Promise<void> {
  return invoke("start_agent_widget_drag", { agent });
}

export function getDisplaySnapshot(): Promise<DisplaySnapshot> {
  return invoke<DisplaySnapshot>("get_display_snapshot");
}

export function setManualDisplayState(state: DisplayState): Promise<DisplaySnapshot> {
  return invoke<DisplaySnapshot>("set_manual_display_state", { state });
}

export function setTrafficLightVisibility(visible: boolean): Promise<void> {
  return invoke("set_traffic_light_visibility", { visible });
}

export function setTrafficLightAlwaysOnTop(enabled: boolean): Promise<void> {
  return invoke("set_traffic_light_always_on_top", { enabled });
}

export function getTrafficLightAlwaysOnTop(): Promise<boolean> {
  return invoke<boolean>("get_traffic_light_always_on_top");
}

export function subscribeToDisplayState(handler: (snapshot: DisplaySnapshot) => void): Promise<UnlistenFn> {
  return listen<DisplaySnapshot>(DISPLAY_STATE_EVENT, (event) => handler(event.payload));
}

export function listSerialPorts(): Promise<SerialPortOption[]> {
  return invoke<SerialPortOption[]>("list_serial_ports");
}

export function getHardwareStatus(): Promise<HardwareStatus> {
  return invoke<HardwareStatus>("get_hardware_status");
}

export function connectSerialDevice(portName: string): Promise<HardwareStatus> {
  return invoke<HardwareStatus>("connect_serial_device", { portName });
}

export function disconnectSerialDevice(): Promise<HardwareStatus> {
  return invoke<HardwareStatus>("disconnect_serial_device");
}

export function testHardwareLed(state: DisplayState): Promise<void> {
  return invoke("test_hardware_led", { displayState: state });
}

export function subscribeToHardwareStatus(handler: (status: HardwareStatus) => void): Promise<UnlistenFn> {
  return listen<HardwareStatus>(HARDWARE_STATUS_EVENT, (event) => handler(event.payload));
}

// Re-export hotzone functions
export {
  getHotzoneMode,
  setHotzoneMode,
  subscribeToHotzoneMode,
  getAgentHotzone,
  setAgentHotzone,
  getActiveHotzoneAt,
  getDefaultHotzone,
  setDefaultHotzone,
  getAgentHotzoneAt,
  getAvailableMonitors,
  getMonitorAt,
  getPeekDelay,
  setPeekDelay,
  getHotzonePresets,
  getPresetHotzone,
  getAgentHotzonePreset,
  setAgentHotzonePreset,
  subscribeToDisplayTopology,
  subscribeToHotzoneWidgetVisibility,
  DISPLAY_TOPOLOGY_EVENT,
  HOTZONE_WIDGET_EVENT,
} from "./hotzone";
export type { HotZone, MonitorInfo, MonitorRect, NormalizedHotZone, HotZonePresetName, HotzoneWidgetVisibility } from "./hotzone";
