import { useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { disable as disableAutostart, enable as enableAutostart, isEnabled as isAutostartEnabled } from "@tauri-apps/plugin-autostart";
import { DISPLAY_META, DISPLAY_STATES, type DisplaySnapshot, type DisplayState } from "../lib/display";
import {
  connectSerialDevice,
  disconnectSerialDevice,
  getHardwareStatus,
  getDisplaySnapshot,
  getSessions,
  getAgentWidgetVisibility,
  setAgentWidgetVisibility,
  getAudioSettings,
  setAudioSettings,
  subscribeToAudioSettings,
  subscribeToAgentAlerts,
  getWidgetAppearance,
  getTrafficLightAlwaysOnTop,
  listSerialPorts,
  setManualDisplayState,
  setTrafficLightAlwaysOnTop,
  setTrafficLightVisibility,
  setWidgetAppearance,
  subscribeToHardwareStatus,
  subscribeToDisplayState,
  testHardwareLed,
  installAgentIntegration,
  uninstallAgentIntegration,
  restoreAgentIntegration,
  getAgentIntegrationDiagnostic,
  focusAgentSession,
  verifyBridge,
  sendTestAgentEvent,
  type IntegrationDiagnostic,
  type BridgeVerification,
  getHotzoneMode,
  setHotzoneMode,
  getAgentHotzone,
  getAvailableMonitors,
  getPeekDelay,
  setPeekDelay,
  getHotzonePresets,
  getPresetHotzone,
  getAgentHotzonePreset,
  setAgentHotzonePreset,
  subscribeToDisplayTopology,
  type HardwareStatus,
  type AgentSession,
  type WidgetAppearance,
  type SerialPortOption,
  type HotZone,
  type MonitorInfo,
  type NormalizedHotZone,
  type MonitorRect,
  type HotZonePresetName as NormalizedHotzonePresetName,
  type AudioAlertSettings,
} from "../lib/tauri";
import { isDoNotDisturbActive, playAudioAlert, previewSound, unlockAudioContext } from "../lib/audio";
import { hasNotificationPermission, requestNotificationPermission, sendDesktopNotification } from "../lib/notification";
import "./app.css";

type AppTab = "overview" | "agents" | "virtual" | "hardware";
const IS_DEV_BUILD = Boolean((import.meta as ImportMeta & { env?: { DEV?: boolean } }).env?.DEV);

function traceStartup<T>(label: string, task: Promise<T>): Promise<T> {
  if (!IS_DEV_BUILD) return task;
  const startedAt = performance.now();
  return task.then(
    (value) => {
      console.info(`[startup] ${label}: ${(performance.now() - startedAt).toFixed(0)}ms`);
      return value;
    },
    (reason: unknown) => {
      console.warn(`[startup] ${label} failed after ${(performance.now() - startedAt).toFixed(0)}ms`, reason);
      throw reason;
    },
  );
}

const INTEGRATION_AGENTS = ["codex", "claude", "cursor", "gemini", "windsurf", "copilot", "antigravity"] as const;
type IntegrationAgent = typeof INTEGRATION_AGENTS[number];

const DEFAULT_AUDIO_ALERT_SETTINGS: AudioAlertSettings = {
  sound_enabled: true,
  sound_volume: 75,
  sound_on_attention: true,
  sound_on_completion: false,
  sound_preset: "gentle_chime",
  notifications_enabled: true,
  do_not_disturb: false,
  do_not_disturb_until: null,
  suppress_when_focused: true,
};

const DND_SNOOZE_OPTIONS = [
  { label: "15 min", minutes: 15 },
  { label: "1 hour", minutes: 60 },
  { label: "4 hours", minutes: 240 },
] as const;

function snoozeUntil(minutes: number): string {
  return new Date(Date.now() + minutes * 60000).toISOString();
}

function formatDoNotDisturbRemaining(settings: AudioAlertSettings, now = Date.now()): string | null {
  if (settings.do_not_disturb) return "Do not disturb is on until you turn it off";
  if (!settings.do_not_disturb_until) return null;
  const until = Date.parse(settings.do_not_disturb_until);
  if (!Number.isFinite(until) || until <= now) return null;
  const minutes = Math.max(1, Math.round((until - now) / 60000));
  if (minutes < 60) return `Snoozed for ${minutes} more min`;
  const hours = Math.floor(minutes / 60);
  const rest = minutes % 60;
  return rest === 0 ? `Snoozed for ${hours}h` : `Snoozed for ${hours}h ${rest}m`;
}

const AGENT_DISPLAY_NAMES: Record<string, string> = {
  codex: "Codex",
  claude: "Claude Code",
  cursor: "Cursor",
  gemini: "Gemini CLI",
  windsurf: "Windsurf",
  copilot: "Copilot",
  antigravity: "Antigravity",
  pi: "Pi",
};

const HOTZONE_PRESET_OPTIONS = [
  { value: "top_left", label: "Top left", gridArea: "top-left" },
  { value: "center_top", label: "Middle top", gridArea: "top-center" },
  { value: "top_right", label: "Top right", gridArea: "top-right" },
  { value: "center_left", label: "Middle left", gridArea: "center-left" },
  { value: "center_right", label: "Middle right", gridArea: "center-right" },
  { value: "bottom_left", label: "Bottom left", gridArea: "bottom-left" },
  { value: "center_bottom", label: "Middle bottom", gridArea: "bottom-center" },
  { value: "bottom_right", label: "Bottom right", gridArea: "bottom-right" },
] as const;

function inferHotzonePreset(hotzone: HotZone, monitor: MonitorInfo): string {
  const localX = hotzone.x - monitor.x;
  const localY = hotzone.y - monitor.y;
  const size = hotzone.width;
  const positions: Record<string, [number, number]> = {
    top_left: [0, 0],
    top_right: [monitor.width - size, 0],
    center_top: [(monitor.width - size) / 2, 0],
    center_left: [0, (monitor.height - size) / 2],
    center_right: [monitor.width - size, (monitor.height - size) / 2],
    center_bottom: [(monitor.width - size) / 2, monitor.height - size],
    bottom_left: [0, monitor.height - size],
    bottom_right: [monitor.width - size, monitor.height - size],
  };
  return Object.entries(positions).reduce((closest, [preset, [x, y]]) =>
    Math.abs(localX - x) + Math.abs(localY - y) < Math.abs(localX - positions[closest][0]) + Math.abs(localY - positions[closest][1])
      ? preset
      : closest, "bottom_left");
}

const NAV_ITEMS: { id: AppTab; label: string; icon: string }[] = [
  { id: "overview", label: "Overview", icon: "overview" },
  { id: "agents", label: "Agents", icon: "agents" },
  { id: "virtual", label: "Virtual light", icon: "display" },
  { id: "hardware", label: "Hardware", icon: "hardware" },
];

async function isDashboardInBackground(): Promise<boolean> {
  try {
    const window = getCurrentWindow();
    const [visible, minimized, focused] = await Promise.all([
      window.isVisible(),
      window.isMinimized(),
      window.isFocused(),
    ]);
    return !visible || minimized || !focused;
  } catch {
    return true;
  }
}

function HotzoneTargetGuide({ hotzone, monitor, positionLabel }: { hotzone: HotZone; monitor: MonitorInfo; positionLabel: string }) {
  const localX = hotzone.x - monitor.x;
  const localY = hotzone.y - monitor.y;

  return (
    <div className="hotzone-target-guide">
      <div className="hotzone-target-copy">
        <p className="eyebrow">WHERE TO MOVE YOUR MOUSE</p>
        <span>Move the pointer into the highlighted area on {monitor.name ?? "this display"}.</span>
      </div>
      <div
        aria-label={`${positionLabel} hotzone on ${monitor.name ?? "display"}`}
        className="hotzone-target-screen"
        role="img"
        style={{ aspectRatio: `${monitor.width} / ${monitor.height}` }}
      >
        <div
          className="hotzone-target-area"
          style={{
            left: `${localX / monitor.width * 100}%`,
            top: `${localY / monitor.height * 100}%`,
            width: `${hotzone.width / monitor.width * 100}%`,
            height: `${hotzone.height / monitor.height * 100}%`,
          }}
        />
        <svg
          aria-hidden="true"
          className="hotzone-target-cursor"
          style={{ left: `${(localX + hotzone.width / 2) / monitor.width * 100}%`, top: `${(localY + hotzone.height / 2) / monitor.height * 100}%` }}
          viewBox="0 0 24 30"
        >
          <path d="M3 2v22l6-6 4 9 4-2-4-9h8L3 2z" />
        </svg>
      </div>
    </div>
  );
}

function TabIcon({ name }: { name: string }) {
  const common = { fill: "none", stroke: "currentColor", strokeWidth: 1.8, strokeLinecap: "round" as const, strokeLinejoin: "round" as const };
  if (name === "agents") return <svg viewBox="0 0 20 20" aria-hidden="true" {...common}><circle cx="7" cy="7" r="3" /><path d="M2.5 16c.6-2.6 2.1-4 4.5-4s3.9 1.4 4.5 4M13 5.2a2.7 2.7 0 0 1 0 5.2m1.1 2c1.7.6 2.8 1.8 3.3 3.6" /></svg>;
  if (name === "display") return <svg viewBox="0 0 20 20" aria-hidden="true" {...common}><rect x="2.5" y="3" width="15" height="11" rx="2" /><path d="M7 17h6m-3-3v3" /></svg>;
  if (name === "hardware") return <svg viewBox="0 0 20 20" aria-hidden="true" {...common}><rect x="4" y="4" width="12" height="12" rx="2" /><path d="M7 1.8v2.1m3-2.1v2.1m3-2.1v2.1m-6 12.2v2.1m3-2.1v2.1m3-2.1v2.1M1.8 7h2.1m-2.1 3h2.1m-2.1 3h2.1m12.2-6h2.1m-2.1 3h2.1m-2.1 3h2.1" /></svg>;
  return <svg viewBox="0 0 20 20" aria-hidden="true" {...common}><path d="M3 15.8V9.5m7 6.3V4m7 11.8v-9" /><path d="m2 7 8-4 8 4" /></svg>;
}

export function App() {
  const [activeTab, setActiveTab] = useState<AppTab>("overview");
  const [agentWidgetVisibility, setAgentWidgetVisibilityState] = useState<Record<string, boolean>>(
    Object.fromEntries([...INTEGRATION_AGENTS, "pi"].map((agent) => [agent, false])),
  );
  const [audioSettings, setAudioSettingsState] = useState<AudioAlertSettings>(DEFAULT_AUDIO_ALERT_SETTINGS);
  const [notificationPermissionGranted, setNotificationPermissionGranted] = useState(false);
  const [alertClock, setAlertClock] = useState(() => Date.now());
  const audioSettingsRef = useRef(DEFAULT_AUDIO_ALERT_SETTINGS);
  const audioSettingsUpdateRef = useRef(0);
  const [snapshot, setSnapshot] = useState<DisplaySnapshot>({ state: "IDLE", source: "manual_preview" });
  const [alwaysOnTop, setAlwaysOnTop] = useState(true);
  const [hotzoneMode, setHotzoneModeState] = useState(false);
  const [editingHotzone, setEditingHotzone] = useState<string | null>(null);
  const [editingHotzoneValue, setEditingHotzoneValue] = useState<HotZone>({ x: 0, y: 0, width: 100, height: 100 });
  const [peekDelay, setPeekDelayState] = useState(2000);
  const [monitors, setMonitors] = useState<MonitorInfo[]>([]);
  const [hotzonePresets, setHotzonePresets] = useState<string[]>([]);
  const [selectedPreset, setSelectedPreset] = useState<string>("bottom_left");
  const [selectedMonitorIndex, setSelectedMonitorIndex] = useState(0);
  const [hotzoneSize, setHotzoneSize] = useState(100);
  const [launchAtLogin, setLaunchAtLogin] = useState(false);
  const [trafficLightVisible, setTrafficLightVisible] = useState(false);
  const [hardwareStatus, setHardwareStatus] = useState<HardwareStatus>({
    connected: false,
    port_name: null,
    device_id: null,
    error: null,
  });
  const [serialPorts, setSerialPorts] = useState<SerialPortOption[]>([]);
  const [serialPortsLoaded, setSerialPortsLoaded] = useState(false);
  const [sessions, setSessions] = useState<AgentSession[]>([]);
  const [widgetAppearance, setWidgetAppearanceState] = useState<WidgetAppearance>({
    size: "standard",
    theme: "dark",
    opacity: 100,
    visual_style: "traffic_light",
    face_pack: "taby_classic",
  });
  const [selectedPort, setSelectedPort] = useState("");
  const [hardwareBusy, setHardwareBusy] = useState(false);
  const [installingAgent, setInstallingAgent] = useState<string | null>(null);
  const [integrationStatus, setIntegrationStatus] = useState<Record<string, string>>({});
  const [installedAgents, setInstalledAgents] = useState<Record<string, boolean>>({});
  const [integrationsLoaded, setIntegrationsLoaded] = useState(false);
  const [diagnostics, setDiagnostics] = useState<Record<string, IntegrationDiagnostic>>({});
  const [testSignalStatus, setTestSignalStatus] = useState<Record<string, string>>({});
  const [bridgeVerification, setBridgeVerification] = useState<BridgeVerification | null>(null);
  const [verifyingBridge, setVerifyingBridge] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const effectStartedAt = performance.now();
    if (IS_DEV_BUILD) console.info("[startup] dashboard effect started");
    let unlisten: (() => void) | undefined;
    let unlistenHardware: (() => void) | undefined;
    let unlistenAudioSettings: (() => void) | undefined;
    let unlistenAgentAlerts: (() => void) | undefined;
    let unlistenTopology: (() => void) | undefined;
    const unlockAudioOnInteraction = () => unlockAudioContext();
    window.addEventListener("pointerdown", unlockAudioOnInteraction, { once: true });

    void Promise.all([
      traceStartup("display snapshot", getDisplaySnapshot()),
      traceStartup("always-on-top preference", getTrafficLightAlwaysOnTop()),
      traceStartup("hardware status", getHardwareStatus()),
      traceStartup("widget appearance", getWidgetAppearance()),
      traceStartup("autostart status", isAutostartEnabled()),
      traceStartup("hotzone mode", getHotzoneMode()),
      traceStartup("peek delay", getPeekDelay()),
      traceStartup("monitor enumeration", getAvailableMonitors()),
      traceStartup("hotzone presets", getHotzonePresets()),
    ])
      .then(([nextSnapshot, nextAlwaysOnTop, nextHardwareStatus, nextAppearance, nextLaunchAtLogin, nextHotzoneMode, nextPeekDelay, nextMonitors, nextPresets]) => {
        setSnapshot(nextSnapshot);
        setAlwaysOnTop(nextAlwaysOnTop);
        setHotzoneModeState(nextHotzoneMode);
        setPeekDelayState(nextPeekDelay);
        setMonitors(nextMonitors);
        setHotzonePresets(nextPresets);
        setHardwareStatus(nextHardwareStatus);
        setWidgetAppearanceState(nextAppearance);
        setLaunchAtLogin(nextLaunchAtLogin);
        if (IS_DEV_BUILD) console.info(`[startup] dashboard initial batch ready: ${(performance.now() - effectStartedAt).toFixed(0)}ms`);
      })
      .catch((reason: unknown) => setError(String(reason)));

    void traceStartup("audio settings", getAudioSettings())
      .then((settings) => {
        audioSettingsRef.current = settings;
        setAudioSettingsState(settings);
      })
      .catch((reason: unknown) => setError(String(reason)));
    void traceStartup("notification permission", hasNotificationPermission()).then(setNotificationPermissionGranted);

    void Promise.all([...INTEGRATION_AGENTS, "pi"].map(async (agent) => [agent, await traceStartup(`widget visibility: ${agent}`, getAgentWidgetVisibility(agent))] as const))
      .then((visibility) => setAgentWidgetVisibilityState(Object.fromEntries(visibility)))
      .catch((reason: unknown) => setError(String(reason)));

    const refreshSessions = (isInitialLoad = false) => {
      const request = getSessions();
      void (isInitialLoad ? traceStartup("initial session list", request) : request)
        .then(setSessions)
        .catch((reason: unknown) => setError(String(reason)));
    };
    refreshSessions(true);
    const sessionRefresh = window.setInterval(refreshSessions, 1500);

    void subscribeToDisplayState(setSnapshot)
      .then((cleanup) => {
        unlisten = cleanup;
      })
      .catch((reason: unknown) => setError(String(reason)));

    void subscribeToHardwareStatus(setHardwareStatus)
      .then((cleanup) => {
        unlistenHardware = cleanup;
      })
      .catch((reason: unknown) => setError(String(reason)));

    void subscribeToAudioSettings((settings) => {
      audioSettingsRef.current = settings;
      setAudioSettingsState(settings);
    }).then((cleanup) => {
      unlistenAudioSettings = cleanup;
    });

    void subscribeToDisplayTopology((rects: MonitorRect[]) => {
      // Refresh display metadata after a monitor is added, removed, or rescaled
      // so the hotzone picker and previews stay accurate.
      void getAvailableMonitors()
        .then((next) => {
          setMonitors(next);
          setSelectedMonitorIndex((current) => (next[current] ? current : 0));
        })
        .catch(() => undefined);
      if (rects.length === 0) setEditingHotzone(null);
    }).then((cleanup) => {
      unlistenTopology = cleanup;
    });

    void subscribeToAgentAlerts((alert) => {
      const currentSettings = audioSettingsRef.current;
      playAudioAlert(
        alert.kind === "completion" ? "completion" : alert.state === "ERROR" ? "error" : "attention",
        currentSettings,
      );
      if (alert.kind === "attention") {
        const agentName = AGENT_DISPLAY_NAMES[alert.agent] ?? alert.agent;
        const stateDetail = alert.state === "ERROR" ? "reported an error and needs attention." : "is waiting for your input.";
        const projectDetail = alert.project ? ` in ${alert.project}` : "";
        const notificationsAllowed = currentSettings.notifications_enabled
          && !isDoNotDisturbActive(currentSettings);
        if (notificationsAllowed) {
          const deliver = () => void sendDesktopNotification(
            `${agentName} needs attention`,
            `${agentName}${projectDetail} ${stateDetail}`,
            true,
          );
          if (currentSettings.suppress_when_focused) {
            void isDashboardInBackground().then((inBackground) => {
              if (inBackground) deliver();
            });
          } else {
            deliver();
          }
        }
      }
    }).then((cleanup) => {
      unlistenAgentAlerts = cleanup;
    });

    return () => {
      window.clearInterval(sessionRefresh);
      window.removeEventListener("pointerdown", unlockAudioOnInteraction);
      unlisten?.();
      unlistenHardware?.();
      unlistenAudioSettings?.();
      unlistenAgentAlerts?.();
      unlistenTopology?.();
    };
  }, []);

  useEffect(() => {
    const timer = window.setInterval(() => {
      setAlertClock(Date.now());
      const current = audioSettingsRef.current;
      if (!current.do_not_disturb_until || current.do_not_disturb) return;
      const until = Date.parse(current.do_not_disturb_until);
      if (!Number.isFinite(until) || until > Date.now()) return;
      const next = { ...current, do_not_disturb_until: null };
      audioSettingsRef.current = next;
      setAudioSettingsState(next);
      void setAudioSettings(next).catch(() => undefined);
    }, 20000);
    return () => window.clearInterval(timer);
  }, []);

  useEffect(() => {
    if (activeTab !== "hardware" || serialPortsLoaded) return;
    let cancelled = false;
    void listSerialPorts()
      .then((ports) => {
        if (cancelled) return;
        setSerialPorts(ports);
        setSerialPortsLoaded(true);
        setSelectedPort((current) => ports.some((port) => port.port_name === current)
          ? current
          : ports[0]?.port_name ?? "");
      })
      .catch((reason: unknown) => {
        if (!cancelled) setError(String(reason));
      });
    return () => { cancelled = true; };
  }, [activeTab, serialPortsLoaded]);

  useEffect(() => {
    if (activeTab !== "agents" || integrationsLoaded) return;
    let cancelled = false;
    void Promise.all(
      INTEGRATION_AGENTS.map(async (agent) => [agent, await getAgentIntegrationDiagnostic(agent)] as const),
    ).then((entries) => {
      if (cancelled) return;
      setDiagnostics(Object.fromEntries(entries));
      setInstalledAgents(Object.fromEntries(entries.map(([agent, diagnostic]) => [agent, diagnostic.installed])));
      setIntegrationsLoaded(true);
    }).catch((reason: unknown) => {
      if (!cancelled) setError(String(reason));
    });
    return () => { cancelled = true; };
  }, [activeTab, integrationsLoaded]);

  async function refreshSerialPorts() {
    try {
      const ports = await listSerialPorts();
      setSerialPorts(ports);
      setSerialPortsLoaded(true);
      if (!ports.some((port) => port.port_name === selectedPort)) {
        setSelectedPort(ports[0]?.port_name ?? "");
      }
      setError(null);
    } catch (reason) {
      setError(String(reason));
    }
  }

  async function toggleHardwareConnection() {
    setHardwareBusy(true);
    try {
      const status = hardwareStatus.connected
        ? await disconnectSerialDevice()
        : await connectSerialDevice(selectedPort);
      setHardwareStatus(status);
      setError(null);
    } catch (reason) {
      setError(String(reason));
    } finally {
      setHardwareBusy(false);
    }
  }

  async function runLedTest(state: DisplayState) {
    try {
      await testHardwareLed(state);
      setError(null);
    } catch (reason) {
      setError(String(reason));
    }
  }

  async function chooseState(state: DisplayState) {
    try {
      setSnapshot(await setManualDisplayState(state));
      setError(null);
    } catch (reason) {
      setError(String(reason));
    }
  }

  async function toggleVisibility() {
    try {
      const nextVisible = !trafficLightVisible;
      await setTrafficLightVisibility(nextVisible);
      setTrafficLightVisible(nextVisible);
      setError(null);
    } catch (reason) {
      setError(String(reason));
    }
  }

  async function setAgentWidgetVisible(agent: string, visible: boolean) {
    try {
      await setAgentWidgetVisibility(agent, visible);
      setAgentWidgetVisibilityState((current) => ({ ...current, [agent]: visible }));
      setError(null);
    } catch (reason) { setError(String(reason)); }
  }

  async function updateAudioSettings(patch: Partial<AudioAlertSettings>) {
    const current = audioSettingsRef.current;
    const next = { ...current, ...patch };
    const updateId = ++audioSettingsUpdateRef.current;
    audioSettingsRef.current = next;
    setAudioSettingsState(next);
    if (next.sound_enabled) unlockAudioContext();

    try {
      const saved = await setAudioSettings(next);
      if (updateId === audioSettingsUpdateRef.current) {
        audioSettingsRef.current = saved;
        setAudioSettingsState(saved);
      }
      setError(null);
    } catch (reason) {
      if (updateId === audioSettingsUpdateRef.current) {
        audioSettingsRef.current = current;
        setAudioSettingsState(current);
      }
      setError(String(reason));
    }
  }

  async function enableDoNotDisturb() {
    await updateAudioSettings({ do_not_disturb: true, do_not_disturb_until: null });
  }

  async function snoozeAlerts(minutes: number) {
    await updateAudioSettings({
      do_not_disturb: false,
      do_not_disturb_until: snoozeUntil(minutes),
    });
  }

  async function clearDoNotDisturb() {
    await updateAudioSettings({ do_not_disturb: false, do_not_disturb_until: null });
  }

  async function enableDesktopNotifications() {
    const granted = await requestNotificationPermission();
    setNotificationPermissionGranted(granted);
    if (!granted) {
      await updateAudioSettings({ notifications_enabled: false });
      setError("Desktop notification permission was not granted by the operating system.");
      return;
    }
    await updateAudioSettings({ notifications_enabled: true });
  }

  async function toggleAlwaysOnTop() {
    try {
      const nextValue = !alwaysOnTop;
      await setTrafficLightAlwaysOnTop(nextValue);
      setAlwaysOnTop(nextValue);
      setError(null);
    } catch (reason) {
      setError(String(reason));
    }
  }

  async function toggleHotzoneMode() {
    try {
      const nextValue = !hotzoneMode;
      await setHotzoneMode(nextValue);
      setHotzoneModeState(nextValue);
      setError(null);
    } catch (reason) {
      setError(String(reason));
    }
  }

  /** Physical size of a logical hotzone on a given display. */
  function physicalHotzoneSize(logical: number, monitorIndex = selectedMonitorIndex): number {
    const scale = monitors[monitorIndex]?.scale_factor ?? 1;
    return Math.max(8, Math.round(logical * scale));
  }

  async function selectHotzonePreset(preset: string, size = hotzoneSize, monitorIndex = selectedMonitorIndex) {
    const monitor = monitors[monitorIndex];
    if (!monitor) return;
    try {
      const physicalSize = physicalHotzoneSize(size, monitorIndex);
      const hotzone = await getPresetHotzone(preset, monitor.width, monitor.height, physicalSize);
      hotzone.x += monitor.x;
      hotzone.y += monitor.y;
      hotzone.monitor_id = monitorIndex;
      setSelectedPreset(preset);
      setHotzoneSize(size);
      setSelectedMonitorIndex(monitorIndex);
      setEditingHotzoneValue(hotzone);
      setError(null);
    } catch (reason) {
      setError(String(reason));
    }
  }

  async function loadAgentHotzone(agent: string) {
    try {
      const preset = await getAgentHotzonePreset(agent);
      if (preset) {
        const monitorIndex = monitors[preset.monitor_index] ? preset.monitor_index : 0;
        await selectHotzonePreset(preset.preset, preset.size_logical, monitorIndex);
      } else {
        const hz = await getAgentHotzone(agent);
        if (hz) {
          const storedMonitorIndex = hz.monitor_id;
          const monitorIndex = storedMonitorIndex !== undefined && monitors[storedMonitorIndex]
            ? storedMonitorIndex
            : Math.max(0, monitors.findIndex((monitor) =>
              hz.x >= monitor.x && hz.x < monitor.x + monitor.width && hz.y >= monitor.y && hz.y < monitor.y + monitor.height));
          const monitor = monitors[monitorIndex];
          if (monitor) {
            const logicalSize = Math.max(50, Math.round(hz.width / (monitor.scale_factor || 1)));
            await selectHotzonePreset(inferHotzonePreset(hz, monitor), logicalSize, monitorIndex);
          } else {
            setEditingHotzoneValue(hz);
            setHotzoneSize(hz.width);
            setSelectedMonitorIndex(monitorIndex);
          }
        } else {
          await selectHotzonePreset(selectedPreset, hotzoneSize, selectedMonitorIndex);
        }
      }
      setEditingHotzone(agent);
      setError(null);
    } catch (reason) {
      setError(String(reason));
    }
  }

  async function saveAgentHotzone() {
    if (!editingHotzone) return;
    try {
      const preset: NormalizedHotZone = {
        monitor_index: selectedMonitorIndex,
        preset: selectedPreset as NormalizedHotzonePresetName,
        size_logical: hotzoneSize,
        offset_logical: [0, 0],
      };
      await setAgentHotzonePreset(editingHotzone, preset);
      setEditingHotzone(null);
      setError(null);
    } catch (reason) {
      setError(String(reason));
    }
  }

  async function handlePeekDelayChange(newDelay: number) {
    try {
      await setPeekDelay(newDelay);
      setPeekDelayState(newDelay);
      setError(null);
    } catch (reason) {
      setError(String(reason));
    }
  }

  async function changeWidgetAppearance(next: WidgetAppearance) {
    try {
      setWidgetAppearanceState(await setWidgetAppearance(next));
      setError(null);
    } catch (reason) {
      setError(String(reason));
    }
  }

  async function toggleLaunchAtLogin() {
    try {
      if (launchAtLogin) await disableAutostart();
      else await enableAutostart();
      setLaunchAtLogin(await isAutostartEnabled());
      setError(null);
    } catch (reason) {
      setError(String(reason));
    }
  }

  function agentStatusClass(agent: string): string {
    const status = diagnostics[agent]?.status_text;
    if (!status) return "agent-status planned";
    if (status.startsWith("Installed & healthy")) return "agent-status supported";
    if (status.startsWith("Out of sync") || status.startsWith("Misconfigured")) return "agent-status warning";
    if (status.startsWith("Installed")) return "agent-status warning";
    return "agent-status planned";
  }

  async function focusSession(agent: string) {
    try {
      const outcome = await focusAgentSession(agent);
      setError(null);
      setTestSignalStatus((current) => ({ ...current, [agent]: outcome.detail }));
      setTimeout(() => {
        setTestSignalStatus((current) => {
          const next = { ...current };
          delete next[agent];
          return next;
        });
      }, 4000);
    } catch (reason) {
      setError(String(reason));
    }
  }

  async function refreshAgentDiagnostics() {
    try {
      const diagList = await Promise.all(
        INTEGRATION_AGENTS.map(async (agent) => [agent, await getAgentIntegrationDiagnostic(agent)] as const)
      );
      setDiagnostics(Object.fromEntries(diagList));
      setInstalledAgents(Object.fromEntries(diagList.map(([agent, diagnostic]) => [agent, diagnostic.installed])));
    } catch {
      // Diagnostic refresh failure is non-blocking
    }
  }

  async function restoreIntegration(agent: string) {
    if (!INTEGRATION_AGENTS.includes(agent as IntegrationAgent)) return;
    setInstallingAgent(agent);
    try {
      const result = await restoreAgentIntegration(agent as IntegrationAgent);
      setIntegrationStatus((current) => ({ ...current, [agent]: result }));
      await refreshAgentDiagnostics();
      setError(null);
    } catch (reason) {
      setError(String(reason));
    } finally {
      setInstallingAgent(null);
    }
  }

  async function checkBridgeHealth() {
    setVerifyingBridge(true);
    try {
      const verification = await verifyBridge();
      setBridgeVerification(verification);
      setError(null);
    } catch (reason) {
      setError(String(reason));
    } finally {
      setVerifyingBridge(false);
    }
  }

  async function installIntegration(agent: string) {
    if (!INTEGRATION_AGENTS.includes(agent as IntegrationAgent)) return;
    const integrationAgent = agent as IntegrationAgent;
    setInstallingAgent(agent);
    try {
      const result = await installAgentIntegration(integrationAgent);
      setIntegrationStatus((current) => ({ ...current, [agent]: result }));
      setInstalledAgents((current) => ({ ...current, [agent]: true }));
      void refreshAgentDiagnostics();
      setError(null);
    } catch (reason) {
      setError(String(reason));
    } finally {
      setInstallingAgent(null);
    }
  }

  async function uninstallIntegration(agent: string) {
    if (!INTEGRATION_AGENTS.includes(agent as IntegrationAgent)) return;
    const integrationAgent = agent as IntegrationAgent;
    setInstallingAgent(agent);
    try {
      const result = await uninstallAgentIntegration(integrationAgent);
      setIntegrationStatus((current) => ({ ...current, [agent]: result }));
      setInstalledAgents((current) => ({ ...current, [agent]: false }));
      void refreshAgentDiagnostics();
      setError(null);
    } catch (reason) {
      setError(String(reason));
    } finally {
      setInstallingAgent(null);
    }
  }

  async function triggerTestSignal(agent: string, eventType: "permission_required" | "agent_working" | "task_completed" | "session_idle") {
    if (!INTEGRATION_AGENTS.includes(agent as IntegrationAgent)) return;
    try {
      await sendTestAgentEvent(agent as IntegrationAgent, eventType);
      const icon = eventType === "permission_required" ? "🔴 Attention" : eventType === "agent_working" ? "🟡 Working" : eventType === "task_completed" ? "🟢 Completed" : "⚪ Idle";
      setTestSignalStatus((current) => ({ ...current, [agent]: `Signal: ${icon}` }));
      setTimeout(() => {
        setTestSignalStatus((current) => {
          const next = { ...current };
          delete next[agent];
          return next;
        });
      }, 3500);
    } catch (reason) {
      setError(String(reason));
    }
  }

  const meta = DISPLAY_META[snapshot.state];
  const workingSessions = sessions.filter((session) => session.state === "WORKING");
  const attentionSessions = sessions.filter((session) => session.state === "WAITING" || session.state === "ERROR");
  const activeSessions = sessions.filter((session) => session.state === "WORKING" || session.state === "WAITING" || session.state === "ERROR");
  const activeAgents = new Set(activeSessions.map((session) => session.agent)).size;
  const doNotDisturbActive = isDoNotDisturbActive(audioSettings, alertClock);
  const doNotDisturbRemaining = formatDoNotDisturbRemaining(audioSettings, alertClock);

  function renderSessionList(emptyText: string) {
    if (sessions.length === 0) return <p className="panel-copy sessions-empty">{emptyText}</p>;
    return (
      <ul className="session-list">
        {sessions.map((session) => (
          <li className="session-row" key={`${session.agent}:${session.session_id}`}>
            <span className={`session-state state-${session.state.toLowerCase()}`} />
            <div className="session-details">
              <strong>{session.project || session.agent}</strong>
              <span>{session.agent} · {session.seconds_since_event}s ago</span>
            </div>
            <span className="session-state-label">{session.state}</span>
            <button
              className="session-focus-button"
              onClick={() => void focusSession(session.agent)}
              title={session.cwd ?? "Focus the agent window"}
              type="button"
            >
              Focus
            </button>
          </li>
        ))}
      </ul>
    );
  }

  const agentCards = [
    { id: "codex", name: "OpenAI Codex", glyph: "C", summary: "Lifecycle hooks · permissions · completions", supported: true },
    { id: "claude", name: "Claude Code", glyph: "✳", summary: "Lifecycle hooks · permissions · errors", supported: true },
    { id: "cursor", name: "Cursor", glyph: "⌁", summary: "Prompt · tool · subagent lifecycle hooks", supported: true },
    { id: "gemini", name: "Gemini CLI", glyph: "✦", summary: "Agent · tool · notification lifecycle hooks", supported: true },
    { id: "windsurf", name: "Windsurf", glyph: "W", summary: "Cascade · tool · workspace lifecycle hooks", supported: true },
    { id: "copilot", name: "GitHub Copilot CLI", glyph: "◉", summary: "Prompt · tool · permission lifecycle hooks", supported: true },
    { id: "antigravity", name: "Antigravity CLI", glyph: "△", summary: "Invocation · tool · completion lifecycle hooks", supported: true },
    { id: "pi", name: "Pi", glyph: "π", summary: "Agent adapter is not implemented yet", supported: false },
  ];

  return (
    <main className="dashboard-shell">
      <header className="app-header" data-tauri-drag-region>
        <div className="brand-lockup" data-tauri-drag-region>
          <span className="brand-mark" aria-hidden="true"><span /><span /><span /></span>
          <span className="brand-name">AgentLight</span>
          <span className="brand-version">0.1.0</span>
        </div>
        <nav className="tab-bar" aria-label="Main sections" role="tablist">
          {NAV_ITEMS.map((item) => (
            <button
              aria-selected={activeTab === item.id}
              className={activeTab === item.id ? "tab-button active" : "tab-button"}
              key={item.id}
              onClick={() => setActiveTab(item.id)}
              role="tab"
              type="button"
            >
              <TabIcon name={item.icon} />
              <span>{item.label}</span>
              {item.id === "agents" && activeAgents > 0 ? <span className="nav-badge">{activeAgents}</span> : null}
            </button>
          ))}
        </nav>
      </header>

      <div className="app-body">
        <section className="workspace-surface">
          <div className="workspace-scroll">
          <div className="tab-view" key={activeTab} role="tabpanel">
      {activeTab === "overview" ? <>
      <div className="page-heading">
        <div><p className="eyebrow">YOUR WORKSPACE</p><h2>Overview</h2></div>
        <span className="workspace-badge"><span className="connection-dot online" />Live local status</span>
      </div>
      <section className="stats-grid" aria-label="Agent activity statistics">
        <article className="stat-card"><span className="stat-label">Active sessions</span><strong>{activeSessions.length}</strong><span className="stat-foot">Across {activeAgents} agents</span></article>
        <article className="stat-card"><span className="stat-label">Working</span><strong className="stat-working">{workingSessions.length}</strong><span className="stat-foot">Processing right now</span></article>
        <article className="stat-card"><span className="stat-label">Needs attention</span><strong className="stat-attention">{attentionSessions.length}</strong><span className="stat-foot">Permission or error</span></article>
        <article className="stat-card"><span className="stat-label">Tracked sessions</span><strong>{sessions.length}</strong><span className="stat-foot">Across configured agents</span></article>
      </section>
      <section className="overview-grid">
        <article className="panel state-panel">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">LIVE STATUS</p>
              <h2>{meta.label}</h2>
            </div>
            <div className={`hero-light-stack state-symbol-${snapshot.state.toLowerCase()}`} aria-hidden="true">
              <span className="hero-bulb red" />
              <span className="hero-bulb yellow" />
              <span className="hero-bulb green" />
            </div>
          </div>
          <div className="hero-bottom-row">
            <p className="panel-copy">{meta.detail}. Your virtual light and connected hardware share this status.</p>
            <div className="state-controls" aria-label="Preview status controls">
              {DISPLAY_STATES.map((state) => (
                <button
                  className={state === snapshot.state ? `state-button active state-${state.toLowerCase()}` : "state-button"}
                  key={state}
                  onClick={() => void chooseState(state)}
                  type="button"
                >
                  {DISPLAY_META[state].label}
                </button>
              ))}
            </div>
          </div>
        </article>

        <article className="panel sessions-panel">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">AGENT SESSIONS</p>
              <h2>{sessions.length} tracked</h2>
            </div>
            <span className="session-agent-badge">All configured agents</span>
          </div>
          {renderSessionList("No agent sessions yet. Start Codex or Claude Code to see activity here.")}
        </article>
      </section>
      </> : null}

      {activeTab === "agents" ? <>
        <div className="page-heading"><div><p className="eyebrow">INTEGRATIONS</p><h2>Agents</h2></div><span className="workspace-badge">{activeAgents} active</span></div>
        <section className="panel bridge-panel">
          <div className="panel-heading">
            <div><p className="eyebrow">LOCAL BRIDGE</p><h2>Diagnostics</h2></div>
            <button className="secondary-button" disabled={verifyingBridge} onClick={() => void checkBridgeHealth()} type="button">
              {verifyingBridge ? "Checking…" : "Verify bridge"}
            </button>
          </div>
          <p className="panel-copy">AgentLight writes hooks natively and keeps a backup before its first change. Verify the loopback bridge to confirm events can reach the app.</p>
          {bridgeVerification ? (
            <div className={`bridge-result${bridgeVerification.reachable && bridgeVerification.token_matches ? " healthy" : " unhealthy"}`}>
              <span className="connection-dot" />
              <div>
                <strong>{bridgeVerification.reachable && bridgeVerification.token_matches ? "Bridge healthy" : "Bridge needs attention"}</strong>
                <span>{bridgeVerification.endpoint}</span>
                <span>{bridgeVerification.detail}</span>
              </div>
            </div>
          ) : (
            <span className="widget-hint">Run a verification to check the endpoint and authentication token.</span>
          )}
        </section>
        <section className="agent-cards">
          {agentCards.map((agent) => {
            const agentSessions = sessions.filter((session) => session.agent === agent.id);
            return (
              <article className="panel agent-card" key={agent.id}>
                <div className="agent-card-top">
                  <div className={`agent-glyph agent-${agent.id}`} aria-hidden="true">{agent.glyph}</div>
                  <span className={agentStatusClass(agent.id)}>
                    {agent.supported
                      ? (!integrationsLoaded ? "Checking…" : diagnostics[agent.id]?.status_text ?? (installedAgents[agent.id] ? "Ready" : "Not installed"))
                      : "Coming later"}
                  </span>
                </div>
                <h3>{agent.name}</h3>
                <p className="panel-copy">{agent.summary}</p>
                <div className="agent-card-meta"><span>{agentSessions.length} tracked</span><span>{agentSessions.filter((session) => session.state === "WORKING").length} working</span></div>
                {agent.supported ? (
                  <div className="agent-card-controls">
                    <div className="agent-card-actions">
                      {installedAgents[agent.id] ? (
                        <button
                          className="secondary-button uninstall-toggle"
                          disabled={installingAgent !== null || !integrationsLoaded}
                          onClick={() => void uninstallIntegration(agent.id)}
                          type="button"
                        >
                          {installingAgent === agent.id ? "Working…" : "Uninstall"}
                        </button>
                      ) : (
                        <button
                          className="secondary-button setup-toggle"
                          disabled={installingAgent !== null || !integrationsLoaded}
                          onClick={() => void installIntegration(agent.id)}
                          type="button"
                        >
                          {installingAgent === agent.id ? "Installing…" : "Install globally"}
                        </button>
                      )}
                    </div>
                    {diagnostics[agent.id]?.config_path ? (
                      <span className="agent-config-hint" title={diagnostics[agent.id].hook_command ?? diagnostics[agent.id].config_path}>
                        {diagnostics[agent.id].config_exists ? "Config: " : "Expected: "}
                        {diagnostics[agent.id].config_path.split(/[/\\]/).slice(-2).join("/")}
                      </span>
                    ) : null}
                    {diagnostics[agent.id]?.backup_available ? (
                      <button
                        className="secondary-button restore-button"
                        disabled={installingAgent !== null}
                        onClick={() => void restoreIntegration(agent.id)}
                        title={diagnostics[agent.id].backup_path ?? undefined}
                        type="button"
                      >
                        Restore backup
                      </button>
                    ) : null}
                    {installedAgents[agent.id] ? (
                      <div className="agent-test-row">
                        <span className="test-row-label">Test signal:</span>
                        <button
                          className="test-btn test-working"
                          onClick={() => void triggerTestSignal(agent.id, "agent_working")}
                          title="Simulate working state (Yellow light)"
                          type="button"
                        >
                          🟡
                        </button>
                        <button
                          className="test-btn test-attention"
                          onClick={() => void triggerTestSignal(agent.id, "permission_required")}
                          title="Simulate permission prompt (Red light)"
                          type="button"
                        >
                          🔴
                        </button>
                        <button
                          className="test-btn test-completed"
                          onClick={() => void triggerTestSignal(agent.id, "task_completed")}
                          title="Simulate task completed (Green light)"
                          type="button"
                        >
                          🟢
                        </button>
                        <button
                          className="test-btn test-idle"
                          onClick={() => void triggerTestSignal(agent.id, "session_idle")}
                          title="Reset to idle"
                          type="button"
                        >
                          ⚪
                        </button>
                      </div>
                    ) : null}
                    {testSignalStatus[agent.id] ? (
                      <div className="test-signal-badge">{testSignalStatus[agent.id]}</div>
                    ) : null}
                    {integrationStatus[agent.id] ? (
                      <div className="setup-instructions">
                        <p>Configuration updated</p>
                        <code>{integrationStatus[agent.id]}</code>
                      </div>
                    ) : null}
                  </div>
                ) : (
                  <p className="widget-hint">Pi sessions will appear here once its adapter is available.</p>
                )}
              </article>
            );
          })}
        </section>
        <section className="panel agent-sessions-panel"><div className="panel-heading"><div><p className="eyebrow">LIVE ACTIVITY</p><h2>Recent sessions</h2></div><span className="session-agent-badge">{sessions.length} tracked</span></div>{renderSessionList("Connected agent sessions will appear here.")}</section>
      </> : null}

      {activeTab === "virtual" ? <>
        <div className="page-heading"><div><p className="eyebrow">OUTPUT & APPEARANCE</p><h2>Virtual lights</h2></div><span className="workspace-badge">One widget per agent</span></div>
        <section className="virtual-layout">
        <article className="panel widget-panel">
          <div className="panel-heading"><div><p className="eyebrow">FLOATING WINDOWS</p><h2>Agent widgets</h2></div><span className="connection-dot online" /></div>
          <p className="panel-copy">Each agent has a separate movable light. Its state follows only that agent’s sessions. Hidden agents stay hidden, including in hotzone mode.</p>
          <div className="agent-widget-list">{agentCards.map((agent) => <div className="agent-widget-row" key={agent.id}><div><strong>{agent.name}</strong><span>{agent.supported ? `${sessions.filter((session) => session.agent === agent.id && ["WORKING", "WAITING", "ERROR"].includes(session.state)).length} active sessions` : "Idle until adapter is available"}</span></div><button className="secondary-button" onClick={() => void setAgentWidgetVisible(agent.id, !agentWidgetVisibility[agent.id])} type="button">{agentWidgetVisibility[agent.id] ? "Hide" : "Show"}</button></div>)}</div>
          <div className="widget-style-controls">
            <label>Widget style
              <select
                onChange={(event) => void changeWidgetAppearance({ ...widgetAppearance, visual_style: event.target.value as WidgetAppearance["visual_style"] })}
                value={widgetAppearance.visual_style}
              >
                <option value="traffic_light">Classic traffic light</option>
                <option value="animated_face">Animated face (Taby)</option>
              </select>
            </label>
            <label>Face set
              <select
                disabled={widgetAppearance.visual_style !== "animated_face"}
                onChange={(event) => void changeWidgetAppearance({ ...widgetAppearance, face_pack: event.target.value as WidgetAppearance["face_pack"] })}
                value={widgetAppearance.face_pack}
              >
                <option value="taby_classic">Classic Taby</option>
                <option value="taby_calm">Calm</option>
                <option value="taby_expressive">Expressive</option>
              </select>
            </label>
          </div>
          {widgetAppearance.visual_style === "animated_face" && (
            <p className="widget-hint face-style-hint">Animated face mode is shared across agent widgets. Switch back to Classic traffic light any time.</p>
          )}
          <div className="appearance-controls">
            <label>Size<select onChange={(event) => void changeWidgetAppearance({ ...widgetAppearance, size: event.target.value as WidgetAppearance["size"] })} value={widgetAppearance.size}><option value="compact">Compact</option><option value="standard">Standard</option><option value="large">Large</option></select></label>
            <label>Housing<select onChange={(event) => void changeWidgetAppearance({ ...widgetAppearance, theme: event.target.value as WidgetAppearance["theme"] })} value={widgetAppearance.theme}><option value="dark">Dark</option><option value="light">Light</option></select></label>
            <label className="opacity-setting">Opacity <span>{widgetAppearance.opacity}%</span><input max={100} min={40} onChange={(event) => void changeWidgetAppearance({ ...widgetAppearance, opacity: Number(event.target.value) })} type="range" value={widgetAppearance.opacity} /></label>
          </div>
          <button className="secondary-button startup-toggle" onClick={() => void toggleLaunchAtLogin()} type="button">{launchAtLogin ? "Launch at login: On" : "Launch at login: Off"}</button>
          <div className="widget-actions">
            <button className="secondary-button" onClick={() => void toggleVisibility()} type="button">{trafficLightVisible ? "Hide virtual lights" : "Show virtual lights"}</button>
            <button className="secondary-button" onClick={() => void toggleAlwaysOnTop()} type="button">{alwaysOnTop ? "Always on top: On" : "Always on top: Off"}</button>
            <button className={`secondary-button${hotzoneMode ? ' active' : ''}`} onClick={() => void toggleHotzoneMode()} type="button">{hotzoneMode ? "Hotzone mode: On" : "Hotzone mode: Off"}</button>
          </div>

          {hotzoneMode && (
            <div className="hotzone-manager">
              <p className="panel-copy">Configure hotzone areas per agent. Move your mouse to a hotzone area to reveal the traffic light.</p>

              {/* Peek delay control */}
              <div className="hotzone-peek-delay">
                <label>Peek delay: {peekDelay}ms
                  <input
                    type="range"
                    min={500}
                    max={5000}
                    step={100}
                    value={peekDelay}
                    onChange={(e) => void handlePeekDelayChange(Number(e.target.value))}
                  />
                </label>
                <span className="widget-hint">How long the traffic light stays visible after mouse leaves the hotzone.</span>
              </div>

              {/* Visual position picker */}
              {editingHotzone && (
                <div className="hotzone-presets">
                  <div className="hotzone-picker-heading">
                    <div>
                      <p className="eyebrow">CHOOSE A HOTZONE POSITION</p>
                      <span className="widget-hint">Pick a preset on your screen—no coordinates needed.</span>
                    </div>
                    {monitors.length > 1 && (
                      <label className="hotzone-monitor-select">Display
                        <select
                          value={selectedMonitorIndex}
                          onChange={(event) => void selectHotzonePreset(selectedPreset, hotzoneSize, Number(event.target.value))}
                        >
                          {monitors.map((monitor, index) => (
                            <option key={index} value={index}>
                              {monitor.name ?? `Monitor ${index + 1}`} · {Math.round(monitor.scale_factor * 100)}%
                            </option>
                          ))}
                        </select>
                      </label>
                    )}
                  </div>
                  <div className="hotzone-position-picker" role="group" aria-label="Hotzone screen position">
                    {HOTZONE_PRESET_OPTIONS.filter(({ value }) => hotzonePresets.length === 0 || hotzonePresets.includes(value)).map((preset) => (
                      <button
                        aria-pressed={selectedPreset === preset.value}
                        className={`hotzone-position-option${selectedPreset === preset.value ? " selected" : ""}`}
                        key={preset.value}
                        onClick={() => void selectHotzonePreset(preset.value)}
                        style={{ gridArea: preset.gridArea }}
                        type="button"
                      >
                        <span className={`position-screen position-${preset.value}`} aria-hidden="true"><i /></span>
                        <span>{preset.label}</span>
                      </button>
                    ))}
                  </div>
                  <label className="hotzone-size-control">Hotzone size <strong>{hotzoneSize} logical px</strong>
                    <input
                      type="range"
                      min={50}
                      max={300}
                      step={10}
                      value={hotzoneSize}
                      onChange={(event) => void selectHotzonePreset(selectedPreset, Number(event.target.value))}
                    />
                  </label>
                  <span className="widget-hint hotzone-dpi-hint">
                    {physicalHotzoneSize(hotzoneSize)} physical px on this display at {Math.round((monitors[selectedMonitorIndex]?.scale_factor ?? 1) * 100)}% scaling. Hotzones follow display moves, resolution changes, and DPI changes automatically.
                  </span>
                </div>
              )}

              {/* Per-agent hotzone list */}
              <div className="agent-widget-list">
                {agentCards.map((agent) => (
                  <div className="agent-widget-row" key={agent.id}>
                    <div>
                      <strong>{agent.name}</strong>
                      <span>{editingHotzone === agent.id ? 'Editing hotzone...' : 'Click to configure hotzone'}</span>
                    </div>
                    <button className="secondary-button" onClick={() => void loadAgentHotzone(agent.id)} type="button">
                      {editingHotzone === agent.id ? 'Editing...' : 'Set hotzone'}
                    </button>
                  </div>
                ))}
              </div>

              {/* Hotzone editor */}
              {editingHotzone && (
                <div className="hotzone-editor">
                  <h3>Hotzone for {agentCards.find(a => a.id === editingHotzone)?.name ?? editingHotzone}</h3>
                  <p className="widget-hint">{HOTZONE_PRESET_OPTIONS.find(({ value }) => value === selectedPreset)?.label} · {hotzoneSize}px · {monitors[selectedMonitorIndex]?.name ?? `Monitor ${selectedMonitorIndex + 1}`}</p>
                  <div className="hotzone-editor-actions">
                    <button className="secondary-button" onClick={() => setEditingHotzone(null)} type="button">Cancel</button>
                    <button className="primary-button" onClick={() => void saveAgentHotzone()} type="button">Save hotzone</button>
                  </div>
                  <p className="widget-hint">Choose a screen position above, then save to use it for this agent.</p>
                </div>
              )}

              {editingHotzone && monitors[selectedMonitorIndex] && (
                <HotzoneTargetGuide
                  hotzone={editingHotzoneValue}
                  monitor={monitors[selectedMonitorIndex]}
                  positionLabel={HOTZONE_PRESET_OPTIONS.find(({ value }) => value === selectedPreset)?.label ?? "Selected position"}
                />
              )}
            </div>
          )}
          <p className="widget-hint">Appearance is shared across the agent lights. Closing this dashboard hides it in the system tray.</p>
        </article>
        <article className="panel preview-panel"><p className="eyebrow">CURRENT DISPLAY</p><div className={`preview-light preview-${snapshot.state.toLowerCase()}`}><i /><i /><i /></div><strong>{meta.label}</strong><span>{meta.detail}</span></article>
        <article className="panel audio-alert-panel">
          <div className="panel-heading">
            <div><p className="eyebrow">AMBIENT ALERTS</p><h2>Sound & notifications</h2></div>
            <span className="workspace-badge">Attention transitions</span>
          </div>
          <p className="panel-copy">Get a subtle alert when an agent needs you. Waiting uses your selected tone, errors use a distinct knock, and repeats are suppressed until the session changes state.</p>
          <div className={`audio-dnd-row${doNotDisturbActive ? " active" : ""}`}>
            <div className="audio-dnd-status">
              <p className="eyebrow">DO NOT DISTURB</p>
              <strong>{doNotDisturbActive ? "Alerts are silenced" : "Alerts are on"}</strong>
              <span>{doNotDisturbRemaining ?? "Sounds and desktop notifications pause while enabled"}</span>
            </div>
            <div className="audio-dnd-actions">
              {doNotDisturbActive ? (
                <button className="secondary-button" onClick={() => void clearDoNotDisturb()} type="button">Turn alerts back on</button>
              ) : (
                <>
                  <button className="secondary-button" onClick={() => void enableDoNotDisturb()} type="button">Until I turn it off</button>
                  {DND_SNOOZE_OPTIONS.map((option) => (
                    <button className="secondary-button" key={option.minutes} onClick={() => void snoozeAlerts(option.minutes)} type="button">
                      Snooze {option.label}
                    </button>
                  ))}
                </>
              )}
            </div>
          </div>
          <div className="audio-settings-layout">
            <div className="audio-toggle-list">
              <label className="audio-setting-toggle">
                <input checked={audioSettings.sound_enabled} onChange={(event) => void updateAudioSettings({ sound_enabled: event.target.checked })} type="checkbox" />
                <span><strong>Sound alerts</strong><small>Master mute for all AgentLight sounds</small></span>
              </label>
              <label className={`audio-setting-toggle${!audioSettings.sound_enabled ? " disabled" : ""}`}>
                <input checked={audioSettings.sound_on_attention} disabled={!audioSettings.sound_enabled} onChange={(event) => void updateAudioSettings({ sound_on_attention: event.target.checked })} type="checkbox" />
                <span><strong>Needs attention</strong><small>Play when an agent waits or reports an error</small></span>
              </label>
              <label className={`audio-setting-toggle${!audioSettings.sound_enabled ? " disabled" : ""}`}>
                <input checked={audioSettings.sound_on_completion} disabled={!audioSettings.sound_enabled} onChange={(event) => void updateAudioSettings({ sound_on_completion: event.target.checked })} type="checkbox" />
                <span><strong>Task completed</strong><small>Optional quiet completion tone</small></span>
              </label>
              <label className="audio-setting-toggle">
                <input checked={audioSettings.notifications_enabled} onChange={(event) => event.target.checked ? void enableDesktopNotifications() : void updateAudioSettings({ notifications_enabled: false })} type="checkbox" />
                <span><strong>Desktop notifications</strong><small>Show a toast when an agent needs attention</small></span>
              </label>
              <label className="audio-setting-toggle">
                <input checked={audioSettings.suppress_when_focused} onChange={(event) => void updateAudioSettings({ suppress_when_focused: event.target.checked })} type="checkbox" />
                <span><strong>Quiet while AgentLight is focused</strong><small>Skip desktop toasts when you are already looking at this dashboard</small></span>
              </label>
              {!notificationPermissionGranted && (
                <button className="secondary-button notification-permission-button" onClick={() => void enableDesktopNotifications()} type="button">Allow system notifications</button>
              )}
            </div>
            <div className="audio-control-group">
              <label>Waiting sound
                <select disabled={!audioSettings.sound_enabled} onChange={(event) => void updateAudioSettings({ sound_preset: event.target.value as AudioAlertSettings["sound_preset"] })} value={audioSettings.sound_preset}>
                  <option value="gentle_chime">Gentle chime</option>
                  <option value="alert_knock">Alert knock</option>
                  <option value="modern_ping">Modern ping</option>
                </select>
              </label>
              <label className="audio-volume-control">Volume <strong>{audioSettings.sound_volume}%</strong>
                <input disabled={!audioSettings.sound_enabled} max={100} min={0} onChange={(event) => void updateAudioSettings({ sound_volume: Number(event.target.value) })} type="range" value={audioSettings.sound_volume} />
              </label>
              <button className="secondary-button audio-preview-button" disabled={!audioSettings.sound_enabled || audioSettings.sound_volume === 0} onClick={() => previewSound(audioSettings.sound_preset, audioSettings.sound_volume)} type="button">Preview sound</button>
            </div>
          </div>
        </article>
        </section>
      </> : null}

      {activeTab === "hardware" ? <>
        <div className="page-heading"><div><p className="eyebrow">OPTIONAL OUTPUT</p><h2>Hardware</h2></div><span className={hardwareStatus.connected ? "workspace-badge online" : "workspace-badge"}><span className="connection-dot" />{hardwareStatus.connected ? "Connected" : "Not connected"}</span></div>
        <section className="hardware-layout">
        <article className="panel hardware-panel">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">OPTIONAL HARDWARE</p>
              <h2>USB serial traffic light</h2>
            </div>
            <span className={hardwareStatus.connected ? "connection-dot online" : "connection-dot"} />
          </div>
          <p className="panel-copy">
            {hardwareStatus.connected
              ? `Connected on ${hardwareStatus.port_name}. The LEDs mirror the resolved status.`
              : hardwareStatus.error
                ? `${hardwareStatus.error}. Connect a compatible microcontroller over USB to mirror the virtual status.`
                : "Connect a compatible microcontroller over USB to mirror the virtual status with three LEDs."}
          </p>
          <div className="hardware-connect-row">
            <select
              aria-label="Serial device"
              disabled={hardwareStatus.connected || hardwareBusy}
              onChange={(event) => setSelectedPort(event.target.value)}
              value={selectedPort}
            >
              {serialPorts.length === 0 ? <option value="">No serial devices found</option> : null}
              {serialPorts.map((port) => (
                <option key={port.port_name} value={port.port_name}>
                  {port.port_name}{port.description ? ` — ${port.description}` : ""}
                </option>
              ))}
            </select>
            <button className="secondary-button" disabled={hardwareBusy} onClick={() => void refreshSerialPorts()} type="button">
              Scan
            </button>
            <button
              className={hardwareStatus.connected ? "secondary-button" : "primary-button"}
              disabled={hardwareBusy || (!hardwareStatus.connected && !selectedPort)}
              onClick={() => void toggleHardwareConnection()}
              type="button"
            >
              {hardwareBusy ? "Connecting…" : hardwareStatus.connected ? "Disconnect" : "Connect"}
            </button>
          </div>
          <div className="hardware-tests" aria-label="LED test controls">
            <button className="state-button" disabled={!hardwareStatus.connected} onClick={() => void runLedTest("WAITING")} type="button">Test red</button>
            <button className="state-button" disabled={!hardwareStatus.connected} onClick={() => void runLedTest("WORKING")} type="button">Test yellow</button>
            <button className="state-button" disabled={!hardwareStatus.connected} onClick={() => void runLedTest("COMPLETED")} type="button">Test green</button>
            <button className="state-button" disabled={!hardwareStatus.connected} onClick={() => void runLedTest("IDLE")} type="button">All off</button>
          </div>
        </article>
        <article className="panel hardware-note"><p className="eyebrow">DEVICE STATUS</p><h3>USB serial traffic light</h3><p className="panel-copy">{hardwareStatus.connected ? `Connected on ${hardwareStatus.port_name}. It mirrors the resolved agent status.` : "Connect a compatible microcontroller over USB to mirror the virtual light. The virtual output continues to work without hardware."}</p><div className="hardware-state-pill"><span className={`session-state state-${snapshot.state.toLowerCase()}`} />Current output: {meta.label}</div></article>
        </section>
      </> : null}
          </div>
          </div>

        </section>
      </div>

      {error ? <p className="error-banner">{error}</p> : null}
    </main>
  );
}
