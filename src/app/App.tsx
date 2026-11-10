import { useEffect, useState } from "react";
import { disable as disableAutostart, enable as enableAutostart, isEnabled as isAutostartEnabled } from "@tauri-apps/plugin-autostart";
import { DISPLAY_META, DISPLAY_STATES, type DisplaySnapshot, type DisplayState } from "../lib/display";
import {
  connectSerialDevice,
  disconnectSerialDevice,
  getHardwareStatus,
  getDisplaySnapshot,
  getSessions,
  setAgentWidgetVisibility,
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
  getAgentIntegrationStatus,
  type HardwareStatus,
  type AgentSession,
  type WidgetAppearance,
  type SerialPortOption,
} from "../lib/tauri";
import "./app.css";

type AppTab = "overview" | "agents" | "virtual" | "hardware";

const INTEGRATION_AGENTS = ["codex", "claude", "cursor", "gemini", "windsurf", "copilot", "antigravity"] as const;
type IntegrationAgent = typeof INTEGRATION_AGENTS[number];

const NAV_ITEMS: { id: AppTab; label: string; icon: string }[] = [
  { id: "overview", label: "Overview", icon: "overview" },
  { id: "agents", label: "Agents", icon: "agents" },
  { id: "virtual", label: "Virtual light", icon: "display" },
  { id: "hardware", label: "Hardware", icon: "hardware" },
];

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
  const [snapshot, setSnapshot] = useState<DisplaySnapshot>({ state: "IDLE", source: "manual_preview" });
  const [alwaysOnTop, setAlwaysOnTop] = useState(true);
  const [launchAtLogin, setLaunchAtLogin] = useState(false);
  const [trafficLightVisible, setTrafficLightVisible] = useState(false);
  const [hardwareStatus, setHardwareStatus] = useState<HardwareStatus>({
    connected: false,
    port_name: null,
    device_id: null,
    error: null,
  });
  const [serialPorts, setSerialPorts] = useState<SerialPortOption[]>([]);
  const [sessions, setSessions] = useState<AgentSession[]>([]);
  const [widgetAppearance, setWidgetAppearanceState] = useState<WidgetAppearance>({ size: "standard", theme: "dark", opacity: 100 });
  const [selectedPort, setSelectedPort] = useState("");
  const [hardwareBusy, setHardwareBusy] = useState(false);
  const [installingAgent, setInstallingAgent] = useState<string | null>(null);
  const [integrationStatus, setIntegrationStatus] = useState<Record<string, string>>({});
  const [installedAgents, setInstalledAgents] = useState<Record<string, boolean>>({});
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let unlistenHardware: (() => void) | undefined;

    void Promise.all([getDisplaySnapshot(), getTrafficLightAlwaysOnTop(), getHardwareStatus(), listSerialPorts(), getWidgetAppearance(), isAutostartEnabled()])
      .then(([nextSnapshot, nextAlwaysOnTop, nextHardwareStatus, nextSerialPorts, nextAppearance, nextLaunchAtLogin]) => {
        setSnapshot(nextSnapshot);
        setAlwaysOnTop(nextAlwaysOnTop);
        setHardwareStatus(nextHardwareStatus);
        setSerialPorts(nextSerialPorts);
        setSelectedPort(nextHardwareStatus.port_name ?? nextSerialPorts[0]?.port_name ?? "");
        setWidgetAppearanceState(nextAppearance);
        setLaunchAtLogin(nextLaunchAtLogin);
      })
      .catch((reason: unknown) => setError(String(reason)));

    void Promise.all(INTEGRATION_AGENTS.map(async (agent) => [agent, await getAgentIntegrationStatus(agent)] as const))
      .then((statuses) => setInstalledAgents(Object.fromEntries(statuses)))
      .catch((reason: unknown) => setError(String(reason)));

    const refreshSessions = () => {
      void getSessions().then(setSessions).catch((reason: unknown) => setError(String(reason)));
    };
    refreshSessions();
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

    return () => {
      window.clearInterval(sessionRefresh);
      unlisten?.();
      unlistenHardware?.();
    };
  }, []);

  async function refreshSerialPorts() {
    try {
      const ports = await listSerialPorts();
      setSerialPorts(ports);
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

  async function installIntegration(agent: string) {
    if (!INTEGRATION_AGENTS.includes(agent as IntegrationAgent)) return;
    const integrationAgent = agent as IntegrationAgent;
    setInstallingAgent(agent);
    try {
      const result = await installAgentIntegration(integrationAgent);
      setIntegrationStatus((current) => ({ ...current, [agent]: result }));
      setInstalledAgents((current) => ({ ...current, [agent]: true }));
      setError(null);
    } catch (reason) {
      setError(String(reason));
    } finally {
      setInstallingAgent(null);
    }
  }

  const meta = DISPLAY_META[snapshot.state];
  const workingSessions = sessions.filter((session) => session.state === "WORKING");
  const attentionSessions = sessions.filter((session) => session.state === "WAITING" || session.state === "ERROR");
  const activeSessions = sessions.filter((session) => session.state === "WORKING" || session.state === "WAITING" || session.state === "ERROR");
  const activeAgents = new Set(activeSessions.map((session) => session.agent)).size;

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
        <section className="agent-cards">
          {agentCards.map((agent) => {
            const agentSessions = sessions.filter((session) => session.agent === agent.id);
            return (
              <article className="panel agent-card" key={agent.id}>
                <div className="agent-card-top">
                  <div className={`agent-glyph agent-${agent.id}`} aria-hidden="true">{agent.glyph}</div>
                  <span className={installedAgents[agent.id] ? "agent-status supported" : "agent-status planned"}>{agent.supported ? (installedAgents[agent.id] ? "Ready" : "Not installed") : "Coming later"}</span>
                </div>
                <h3>{agent.name}</h3>
                <p className="panel-copy">{agent.summary}</p>
                <div className="agent-card-meta"><span>{agentSessions.length} tracked</span><span>{agentSessions.filter((session) => session.state === "WORKING").length} working</span></div>
                {agent.supported ? <>
                  <button className="secondary-button setup-toggle" disabled={installingAgent !== null} onClick={() => void installIntegration(agent.id)} type="button">
                    {installingAgent === agent.id ? "Installing…" : installedAgents[agent.id] ? "Installed globally" : "Install globally"}
                  </button>
                  {integrationStatus[agent.id] ? <div className="setup-instructions"><p>Global configuration updated</p><code>{integrationStatus[agent.id]}</code><span>Applies to every project opened with this agent.</span></div> : null}
                </> : <p className="widget-hint">Pi sessions will appear here once its adapter is available.</p>}
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
          <p className="panel-copy">Each agent has a separate movable light. Its state follows only that agent’s sessions.</p>
          <div className="agent-widget-list">{agentCards.map((agent) => <div className="agent-widget-row" key={agent.id}><div><strong>{agent.name}</strong><span>{agent.supported ? `${sessions.filter((session) => session.agent === agent.id && ["WORKING", "WAITING", "ERROR"].includes(session.state)).length} active sessions` : "Idle until adapter is available"}</span></div><button className="secondary-button" onClick={() => void setAgentWidgetVisible(agent.id, !agentWidgetVisibility[agent.id])} type="button">{agentWidgetVisibility[agent.id] ? "Hide" : "Show"}</button></div>)}</div>
          <div className="appearance-controls">
            <label>Size<select onChange={(event) => void changeWidgetAppearance({ ...widgetAppearance, size: event.target.value as WidgetAppearance["size"] })} value={widgetAppearance.size}><option value="compact">Compact</option><option value="standard">Standard</option><option value="large">Large</option></select></label>
            <label>Housing<select onChange={(event) => void changeWidgetAppearance({ ...widgetAppearance, theme: event.target.value as WidgetAppearance["theme"] })} value={widgetAppearance.theme}><option value="dark">Dark</option><option value="light">Light</option></select></label>
            <label className="opacity-setting">Opacity <span>{widgetAppearance.opacity}%</span><input max={100} min={40} onChange={(event) => void changeWidgetAppearance({ ...widgetAppearance, opacity: Number(event.target.value) })} type="range" value={widgetAppearance.opacity} /></label>
          </div>
          <button className="secondary-button startup-toggle" onClick={() => void toggleLaunchAtLogin()} type="button">{launchAtLogin ? "Launch at login: On" : "Launch at login: Off"}</button>
          <div className="widget-actions">
            <button className="secondary-button" onClick={() => void toggleVisibility()} type="button">{trafficLightVisible ? "Hide virtual lights" : "Show virtual lights"}</button>
            <button className="secondary-button" onClick={() => void toggleAlwaysOnTop()} type="button">{alwaysOnTop ? "Always on top: On" : "Always on top: Off"}</button>
          </div>
          <p className="widget-hint">Appearance is shared across the agent lights. Closing this dashboard hides it in the system tray.</p>
        </article>
        <article className="panel preview-panel"><p className="eyebrow">CURRENT DISPLAY</p><div className={`preview-light preview-${snapshot.state.toLowerCase()}`}><i /><i /><i /></div><strong>{meta.label}</strong><span>{meta.detail}</span></article>
        </section>
      </> : null}

      {activeTab === "hardware" ? <>
        <div className="page-heading"><div><p className="eyebrow">OPTIONAL OUTPUT</p><h2>Hardware</h2></div><span className={hardwareStatus.connected ? "workspace-badge online" : "workspace-badge"}><span className="connection-dot" />{hardwareStatus.connected ? "Connected" : "Not connected"}</span></div>
        <section className="hardware-layout">
        <article className="panel hardware-panel">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">OPTIONAL HARDWARE</p>
              <h2>ESP8266 traffic light</h2>
            </div>
            <span className={hardwareStatus.connected ? "connection-dot online" : "connection-dot"} />
          </div>
          <p className="panel-copy">
            {hardwareStatus.connected
              ? `Connected on ${hardwareStatus.port_name}. The LEDs mirror the resolved status.`
              : hardwareStatus.error
                ? `${hardwareStatus.error}. Connect a NodeMCU over USB to mirror the virtual status.`
                : "Connect a NodeMCU over USB to mirror the virtual status with three LEDs."}
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
        <article className="panel hardware-note"><p className="eyebrow">DEVICE STATUS</p><h3>ESP8266 traffic light</h3><p className="panel-copy">{hardwareStatus.connected ? `Connected on ${hardwareStatus.port_name}. It mirrors the resolved agent status.` : "Connect a NodeMCU over USB to mirror the virtual light. The virtual output continues to work without hardware."}</p><div className="hardware-state-pill"><span className={`session-state state-${snapshot.state.toLowerCase()}`} />Current output: {meta.label}</div></article>
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
