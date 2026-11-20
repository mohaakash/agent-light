import type { DisplayState } from "./display";
import type { FacePack } from "./tauri";

export interface FaceAnimation {
  id: string;
  src: string;
  posterSrc: string;
  durationMs: number;
  loop: boolean;
}

export interface FaceStateAnimation {
  play: FaceAnimation;
  fallback: FaceAnimation;
}

const animation = (id: string, file: string, durationMs: number, loop: boolean): FaceAnimation => ({
  id,
  src: `/assets/taby/a/${file}.gif`,
  posterSrc: `/assets/taby/posters/${id}.webp`,
  durationMs,
  loop,
});

// Curated from firmware-taby/assets/amoled-1.64/catalog.json (pack 0.4.1).
// Keep durations and loop policies synchronized with that source catalog.
const ANIMATIONS = {
  idle: animation("idle_01_loop", "5e63dbe6", 9042, true),
  idleCalm: animation("calm_idle_01_f000", "684df770", 1000, true),
  idleExpressive: animation("idle_02_loop", "523e19a5", 7875, true),
  working: animation("working_laptop_loop", "cdd1dbdc", 2667, true),
  workingCalm: animation("calm_work_medium_f000", "d18417a7", 1000, true),
  workingExpressive: animation("working_laptop_excited_loop", "8d8c83cf", 1667, true),
  waiting: animation("waiting_01", "afd018ca", 12917, false),
  attention: animation("angry_01_loop", "342748ab", 2542, true),
  error: animation("angry_02_loop", "ab6244d8", 2500, true),
  completed: animation("task_completed", "2a3d7c56", 3125, false),
  completedCalm: animation("confirmation", "72811854", 1958, false),
  completedExpressive: animation("thumbs_up", "2620c9f8", 3167, false),
} satisfies Record<string, FaceAnimation>;

const FACE_PACKS: Record<FacePack, Record<DisplayState, FaceStateAnimation>> = {
  taby_classic: {
    IDLE: { play: ANIMATIONS.idle, fallback: ANIMATIONS.idle },
    WORKING: { play: ANIMATIONS.working, fallback: ANIMATIONS.idle },
    WAITING: { play: ANIMATIONS.waiting, fallback: ANIMATIONS.attention },
    ERROR: { play: ANIMATIONS.error, fallback: ANIMATIONS.error },
    COMPLETED: { play: ANIMATIONS.completed, fallback: ANIMATIONS.idle },
  },
  taby_calm: {
    IDLE: { play: ANIMATIONS.idleCalm, fallback: ANIMATIONS.idleCalm },
    WORKING: { play: ANIMATIONS.workingCalm, fallback: ANIMATIONS.idleCalm },
    WAITING: { play: ANIMATIONS.attention, fallback: ANIMATIONS.attention },
    ERROR: { play: ANIMATIONS.error, fallback: ANIMATIONS.error },
    COMPLETED: { play: ANIMATIONS.completedCalm, fallback: ANIMATIONS.idleCalm },
  },
  taby_expressive: {
    IDLE: { play: ANIMATIONS.idleExpressive, fallback: ANIMATIONS.idleExpressive },
    WORKING: { play: ANIMATIONS.workingExpressive, fallback: ANIMATIONS.idleExpressive },
    WAITING: { play: ANIMATIONS.attention, fallback: ANIMATIONS.attention },
    ERROR: { play: ANIMATIONS.error, fallback: ANIMATIONS.error },
    COMPLETED: { play: ANIMATIONS.completedExpressive, fallback: ANIMATIONS.idleExpressive },
  },
};

export function getFaceStateAnimation(state: DisplayState, pack: FacePack): FaceStateAnimation {
  return FACE_PACKS[pack][state];
}
