import { useEffect, useState, useSyncExternalStore, type CSSProperties } from "react";
import type { DisplayState } from "../../lib/display";
import { getFaceStateAnimation } from "../../lib/animated-face";
import type { FacePack, WidgetAppearance } from "../../lib/tauri";

const REDUCED_MOTION_QUERY = "(prefers-reduced-motion: reduce)";

function subscribeToReducedMotion(onChange: () => void) {
  if (typeof window === "undefined") return () => undefined;
  const query = window.matchMedia(REDUCED_MOTION_QUERY);
  query.addEventListener("change", onChange);
  return () => query.removeEventListener("change", onChange);
}

function getReducedMotionSnapshot() {
  return typeof window !== "undefined" && window.matchMedia(REDUCED_MOTION_QUERY).matches;
}

interface AnimatedFaceWidgetProps {
  state: DisplayState;
  pack: FacePack;
  appearance: WidgetAppearance;
  anchor: string;
  onAssetError: () => void;
}

export function AnimatedFaceWidget({ state, pack, appearance, anchor, onAssetError }: AnimatedFaceWidgetProps) {
  const spec = getFaceStateAnimation(state, pack);
  const reducedMotion = useSyncExternalStore(subscribeToReducedMotion, getReducedMotionSnapshot, () => false);
  const [oneShotFinished, setOneShotFinished] = useState(false);

  useEffect(() => {
    if (spec.play.loop || reducedMotion) return;
    const timer = window.setTimeout(() => setOneShotFinished(true), spec.play.durationMs);
    return () => window.clearTimeout(timer);
  }, [spec, reducedMotion]);

  const currentAnimation = oneShotFinished ? spec.fallback : spec.play;

  return (
    <div
      className={`animated-face-notch animated-face-${pack}`}
      data-theme={appearance.theme}
      data-hotzone-anchor={anchor}
      style={{ "--widget-opacity": appearance.opacity / 100 } as CSSProperties}
    >
      <div className="animated-face-notch-body">
        <img
          alt={`Taby face: ${state.toLowerCase()}`}
          className="animated-face-image"
          draggable={false}
          onError={onAssetError}
          src={reducedMotion ? currentAnimation.posterSrc : currentAnimation.src}
        />
      </div>
    </div>
  );
}
