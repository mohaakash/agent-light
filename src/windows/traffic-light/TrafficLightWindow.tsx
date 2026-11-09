import { useEffect, useState, type CSSProperties, type PointerEvent } from "react";
import { DISPLAY_META, type DisplaySnapshot } from "../../lib/display";
import {
  getDisplaySnapshot,
  getAgentDisplaySnapshot,
  getWidgetAppearance,
  subscribeToDisplayState,
  subscribeToWidgetAppearance,
  startAgentWidgetDrag,
  type WidgetAppearance,
} from "../../lib/tauri";
import "./traffic-light.css";

const LIGHTS = [
  { state: "WAITING" as const, className: "red" },
  { state: "WORKING" as const, className: "yellow" },
  { state: "COMPLETED" as const, className: "green" },
];

export function TrafficLightWindow() {
  const [snapshot, setSnapshot] = useState<DisplaySnapshot>({ state: "IDLE", source: "manual_preview" });
  const [appearance, setAppearance] = useState<WidgetAppearance>({ size: "standard", theme: "dark", opacity: 100 });

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let unlistenAppearance: (() => void) | undefined;

    const agent = new URLSearchParams(window.location.search).get("agent");
    if (agent) {
      const refresh = () => void getAgentDisplaySnapshot(agent).then(setSnapshot).catch(() => undefined);
      refresh();
      const timer = window.setInterval(refresh, 900);
      void getWidgetAppearance().then(setAppearance).catch(() => undefined);
      void subscribeToWidgetAppearance(setAppearance).then((cleanup) => { unlistenAppearance = cleanup; });
      return () => { window.clearInterval(timer); unlistenAppearance?.(); };
    }
    void getDisplaySnapshot().then(setSnapshot).catch(() => undefined);
    void getWidgetAppearance().then(setAppearance).catch(() => undefined);
    void subscribeToDisplayState(setSnapshot).then((cleanup) => {
      unlisten = cleanup;
    });
    void subscribeToWidgetAppearance(setAppearance).then((cleanup) => {
      unlistenAppearance = cleanup;
    });

    return () => {
      unlisten?.();
      unlistenAppearance?.();
    };
  }, []);

  const stateLabel = DISPLAY_META[snapshot.state].label;

  function startWindowDrag(event: PointerEvent<HTMLElement>) {
    if (event.button !== 0 || event.isPrimary === false) {
      return;
    }

    const agent = new URLSearchParams(window.location.search).get("agent");
    if (!agent) return;
    void startAgentWidgetDrag(agent).catch((reason: unknown) => {
      console.error("Unable to start dragging the traffic-light window", reason);
    });
  }

  return (
    <main
      aria-label={`AgentLight status: ${stateLabel}`}
      className="traffic-light-stage"
      data-tauri-drag-region
      onPointerDown={startWindowDrag}
      role="status"
      style={{
        "--widget-scale": appearance.size === "compact" ? 0.8 : appearance.size === "large" ? 1.2 : 1,
        "--widget-opacity": appearance.opacity / 100,
      } as CSSProperties}
      data-theme={appearance.theme}
    >
      <div className="traffic-light-housing" data-tauri-drag-region>
        <div className="traffic-light-lights" data-tauri-drag-region>
          {LIGHTS.map(({ state, className }) => (
            <span
              aria-label={DISPLAY_META[state].label}
              className={snapshot.state === state || (snapshot.state === "ERROR" && state === "WAITING") ? `light ${className} active` : `light ${className}`}
              key={state}
              role="img"
            />
          ))}
        </div>
        <span className="traffic-light-label">{stateLabel}</span>
      </div>
    </main>
  );
}
