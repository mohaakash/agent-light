import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { DisplaySnapshot, DisplayState } from "./display";

export const DISPLAY_STATE_EVENT = "display-state-changed";
export const HARDWARE_STATUS_EVENT = "hardware-status-changed";
export const WIDGET_APPEARANCE_EVENT = "widget-appearance-changed";

export type WidgetSize = "compact" | "standard" | "large";
export type WidgetTheme = "dark" | "light";

export interface WidgetAppearance {
  size: WidgetSize;
  theme: WidgetTheme;
  opacity: number;
}

export function getWidgetAppearance(): Promise<WidgetAppearance> {
  return invoke<WidgetAppearance>("get_widget_appearance");
}

export function setWidgetAppearance(appearance: WidgetAppearance): Promise<WidgetAppearance> {
  return invoke<WidgetAppearance>("set_widget_appearance", {
    size: appearance.size,
    theme: appearance.theme,
    opacity: appearance.opacity,
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
}

export function getSessions(): Promise<AgentSession[]> {
  return invoke<AgentSession[]>("get_sessions");
}

export function installAgentIntegration(agent: "codex" | "claude" | "cursor" | "gemini" | "windsurf" | "copilot" | "antigravity"): Promise<string> {
  return invoke<string>("install_agent_integration", { agent });
}

export function getAgentIntegrationStatus(agent: "codex" | "claude" | "cursor" | "gemini" | "windsurf" | "copilot" | "antigravity"): Promise<boolean> {
  return invoke<boolean>("get_agent_integration_status", { agent });
}

export function getAgentDisplaySnapshot(agent: string): Promise<DisplaySnapshot> {
  return invoke<DisplaySnapshot>("get_agent_display_snapshot", { agent });
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
