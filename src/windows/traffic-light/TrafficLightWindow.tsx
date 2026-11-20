import { useCallback, useEffect, useRef, useState, type CSSProperties } from "react";
import { DISPLAY_META, type DisplaySnapshot } from "../../lib/display";
import {
  getDisplaySnapshot,
  getAgentDisplaySnapshot,
  getWidgetAppearance,
  subscribeToDisplayState,
  subscribeToWidgetAppearance,
  startAgentWidgetDrag,
  focusAgentSession,
  subscribeToHotzoneWidgetVisibility,
  subscribeToHotzoneMode,
  type WidgetAppearance,
} from "../../lib/tauri";
import "./traffic-light.css";
import "./hotzone.css";
import "./animated-face.css";
import { AnimatedFaceWidget } from "./AnimatedFaceWidget";

const LIGHTS = [
  { state: "WAITING" as const, className: "red" },
  { state: "WORKING" as const, className: "yellow" },
  { state: "COMPLETED" as const, className: "green" },
];

// The agent is fixed for a widget window and never changes at runtime.
const WIDGET_AGENT = new URLSearchParams(window.location.search).get("agent");

export function TrafficLightWindow() {
  const [snapshot, setSnapshot] = useState<DisplaySnapshot>({ state: "IDLE", source: "manual_preview" });
  const [appearance, setAppearance] = useState<WidgetAppearance>({
    size: "standard",
    theme: "dark",
    opacity: 100,
    visual_style: "traffic_light",
    face_pack: "taby_classic",
  });
  const [failedFaceKey, setFailedFaceKey] = useState<string | null>(null);
  const [hotzoneMotion, setHotzoneMotion] = useState<"enter" | "exit" | null>(null);
  const [hotzoneAnchor, setHotzoneAnchor] = useState("center_top");

  const pointerStartRef = useRef<{ x: number; y: number; moved: boolean } | null>(null);
  const hotzoneMotionTimerRef = useRef<number | null>(null);
  const triggerHotzoneMotion = useCallback((visible: boolean) => {
    if (hotzoneMotionTimerRef.current !== null) {
      window.clearTimeout(hotzoneMotionTimerRef.current);
    }
    setHotzoneMotion(visible ? "enter" : "exit");
    // Keep the completed exit clip in place while the native window hides. If
    // the class is removed at the same moment as window.hide(), it can flash
    // back to its unclipped state for one frame. A new entry or mode-off event
    // clears it before the next reveal.
    if (visible) {
      hotzoneMotionTimerRef.current = window.setTimeout(() => {
        setHotzoneMotion(null);
        hotzoneMotionTimerRef.current = null;
      }, 850);
    }
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let unlistenAppearance: (() => void) | undefined;
    let unlistenHotzone: (() => void) | undefined;
    let unlistenHotzoneMode: (() => void) | undefined;

    if (WIDGET_AGENT) {
      const agent = WIDGET_AGENT;
      const refresh = () => {
        void getAgentDisplaySnapshot(agent).then(setSnapshot).catch(() => undefined);
      };
      refresh();
      const timer = window.setInterval(refresh, 900);
      void getWidgetAppearance().then(setAppearance).catch(() => undefined);
      void subscribeToWidgetAppearance(setAppearance).then((cleanup) => { unlistenAppearance = cleanup; });
      void subscribeToHotzoneWidgetVisibility((visibility) => {
        if (visibility.agent === agent) {
          setHotzoneAnchor(visibility.anchor);
          triggerHotzoneMotion(visibility.visible);
        }
      }).then((cleanup) => { unlistenHotzone = cleanup; });
      void subscribeToHotzoneMode((enabled) => {
        if (!enabled) {
          setHotzoneAnchor("center_top");
          setHotzoneMotion(null);
          if (hotzoneMotionTimerRef.current !== null) {
            window.clearTimeout(hotzoneMotionTimerRef.current);
            hotzoneMotionTimerRef.current = null;
          }
        }
      }).then((cleanup) => { unlistenHotzoneMode = cleanup; });
      return () => {
        window.clearInterval(timer);
        unlistenAppearance?.();
        unlistenHotzone?.();
        unlistenHotzoneMode?.();
        if (hotzoneMotionTimerRef.current !== null) window.clearTimeout(hotzoneMotionTimerRef.current);
      };
    }

    void getDisplaySnapshot().then(setSnapshot).catch(() => undefined);
    void getWidgetAppearance().then(setAppearance).catch(() => undefined);
    void subscribeToDisplayState(setSnapshot).then((cleanup) => { unlisten = cleanup; });
    void subscribeToWidgetAppearance(setAppearance).then((cleanup) => { unlistenAppearance = cleanup; });
    return () => {
      unlisten?.();
      unlistenAppearance?.();
    };
  }, [triggerHotzoneMotion]);

  const faceKey = `${appearance.visual_style}:${appearance.face_pack}`;
  const faceAssetFailed = failedFaceKey === faceKey;
  const focusable = Boolean(WIDGET_AGENT && snapshot.state !== "IDLE");

  async function focusAgent() {
    const agent = WIDGET_AGENT;
    if (!agent || !focusable) return;
    try {
      await focusAgentSession(agent);
    } catch (reason) {
      console.warn("Unable to focus the agent session", reason);
    }
  }

  function handlePointerDown(event: React.PointerEvent<HTMLElement>) {
    if (event.button !== 0 || event.isPrimary === false) return;
    pointerStartRef.current = { x: event.screenX, y: event.screenY, moved: false };
  }

  function handlePointerMove(event: React.PointerEvent<HTMLElement>) {
    const start = pointerStartRef.current;
    if (!start || start.moved) return;
    if (Math.hypot(event.screenX - start.x, event.screenY - start.y) < 4) return;
    start.moved = true;
    const agent = WIDGET_AGENT;
    if (!agent) return;
    void startAgentWidgetDrag(agent).catch((reason: unknown) => {
      console.error("Unable to start dragging the traffic-light window", reason);
    });
  }

  function handlePointerUp() {
    const start = pointerStartRef.current;
    pointerStartRef.current = null;
    if (!start || start.moved) return;
    void focusAgent();
  }

  const stateLabel = DISPLAY_META[snapshot.state].label;

  return (
    <main
      aria-label={`AgentLight status: ${stateLabel}`}
      className="traffic-light-stage"
      data-tauri-drag-region
      data-focusable={focusable}
      data-hotzone-motion={hotzoneMotion ?? undefined}
      data-hotzone-anchor={hotzoneAnchor}
      onPointerDown={handlePointerDown}
      onPointerMove={handlePointerMove}
      onPointerUp={handlePointerUp}
      onPointerLeave={() => { pointerStartRef.current = null; }}
      role="status"
      style={{
        "--widget-scale": appearance.size === "compact" ? 0.8 : appearance.size === "large" ? 1.2 : 1,
        "--widget-opacity": appearance.opacity / 100,
      } as CSSProperties}
      data-theme={appearance.theme}
    >
      {appearance.visual_style === "animated_face" && !faceAssetFailed ? (
        <AnimatedFaceWidget
          key={`${snapshot.state}:${appearance.face_pack}`}
          state={snapshot.state}
          pack={appearance.face_pack}
          appearance={appearance}
          anchor={hotzoneAnchor}
          onAssetError={() => setFailedFaceKey(faceKey)}
        />
      ) : (
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
      )}
    </main>
  );
}
