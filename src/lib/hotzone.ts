import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export const HOTZONE_MODE_EVENT = "hotzone-mode-changed";
export const HOTZONE_WIDGET_EVENT = "hotzone-widget-visibility-changed";

export interface HotzoneWidgetVisibility {
  agent: string;
  visible: boolean;
  anchor: string;
  hotzone: HotZone;
}

export function subscribeToHotzoneWidgetVisibility(
  handler: (visibility: HotzoneWidgetVisibility) => void,
): Promise<UnlistenFn> {
  return listen<HotzoneWidgetVisibility>(HOTZONE_WIDGET_EVENT, (event) => handler(event.payload));
}

export interface HotZone {
  x: number;
  y: number;
  width: number;
  height: number;
  monitor_id?: number;
}

export interface MonitorInfo {
  x: number;
  y: number;
  width: number;
  height: number;
  scale_factor: number;
  name: string | null;
}

export type HotZonePresetName =
  | "top_left"
  | "top_right"
  | "center_left"
  | "center_right"
  | "bottom_left"
  | "bottom_right"
  | "center_top"
  | "center_bottom";

/** Resolution-independent hotzone definition stored by the backend. */
export interface NormalizedHotZone {
  monitor_index: number;
  preset: HotZonePresetName;
  size_logical: number;
  offset_logical: [number, number];
}

export function getAgentHotzonePreset(agent: string): Promise<NormalizedHotZone | null> {
  return invoke<NormalizedHotZone | null>("get_agent_hotzone_preset", { agent });
}

export function setAgentHotzonePreset(agent: string, preset: NormalizedHotZone): Promise<void> {
  return invoke("set_agent_hotzone_preset", { agent, preset });
}

export const DISPLAY_TOPOLOGY_EVENT = "display-topology-changed";

export function subscribeToDisplayTopology(
  handler: (monitors: MonitorRect[]) => void,
): Promise<UnlistenFn> {
  return listen<MonitorRect[]>(DISPLAY_TOPOLOGY_EVENT, (event) => handler(event.payload));
}

/** Physical display geometry, matching the Rust `MonitorRect` type. */
export interface MonitorRect {
  x: number;
  y: number;
  width: number;
  height: number;
  scale_factor: number;
}

/**
 * Get whether hotzone mode is enabled
 */
export function getHotzoneMode(): Promise<boolean> {
  return invoke<boolean>("get_hotzone_mode");
}

/**
 * Set whether hotzone mode is enabled
 */
export function setHotzoneMode(enabled: boolean): Promise<void> {
  return invoke("set_hotzone_mode", { enabled });
}

/**
 * Subscribe to hotzone mode changes
 */
export function subscribeToHotzoneMode(handler: (enabled: boolean) => void): Promise<UnlistenFn> {
  return listen<boolean>(HOTZONE_MODE_EVENT, (event) => handler(event.payload));
}

/**
 * Get hotzone configuration for a specific agent
 */
export function getAgentHotzone(agent: string): Promise<HotZone | null> {
  return invoke<HotZone | null>("get_agent_hotzone", { agent });
}

/**
 * Set hotzone configuration for a specific agent
 */
export function setAgentHotzone(agent: string, hotzone: HotZone): Promise<void> {
  return invoke("set_agent_hotzone", { agent, hotzone });
}

/**
 * Check if a point (x, y) is within any hotzone
 */
export function getActiveHotzoneAt(x: number, y: number): Promise<HotZone | null> {
  return invoke<HotZone | null>("get_active_hotzone_at", { x, y });
}

/**
 * Get the default hotzone configuration (for the main traffic light)
 */
export function getDefaultHotzone(): Promise<HotZone> {
  return invoke<HotZone>("get_default_hotzone");
}

/**
 * Set the default hotzone configuration
 */
export function setDefaultHotzone(hotzone: HotZone): Promise<void> {
  return invoke("set_default_hotzone", { hotzone });
}

/**
 * Check if a point (x, y) is within a specific agent's hotzone
 */
export function getAgentHotzoneAt(agent: string, x: number, y: number): Promise<boolean> {
  return invoke<boolean>("get_agent_hotzone_at", { agent, x, y });
}

/**
 * Get all available monitors with their dimensions and positions
 */
export function getAvailableMonitors(): Promise<MonitorInfo[]> {
  return invoke<MonitorInfo[]>("get_available_monitors");
}

/**
 * Get which monitor contains a point (x, y). Returns the monitor index.
 */
export function getMonitorAt(x: number, y: number): Promise<number | null> {
  return invoke<number | null>("get_monitor_at", { x, y });
}

/**
 * Get the current peek delay in milliseconds
 */
export function getPeekDelay(): Promise<number> {
  return invoke<number>("get_peek_delay");
}

/**
 * Set the peek delay in milliseconds (100-10000)
 */
export function setPeekDelay(delayMs: number): Promise<void> {
  return invoke("set_peek_delay", { delayMs });
}

/**
 * Get available hotzone preset names
 */
export function getHotzonePresets(): Promise<string[]> {
  return invoke<string[]>("get_hotzone_presets");
}

/**
 * Calculate a preset hotzone position relative to a monitor
 */
export function getPresetHotzone(
  preset: string,
  monitorWidth: number,
  monitorHeight: number,
  hotzoneSize: number,
): Promise<HotZone> {
  return invoke<HotZone>("get_preset_hotzone", { preset, monitorWidth, monitorHeight, hotzoneSize });
}