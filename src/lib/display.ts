export type DisplayState = "WAITING" | "WORKING" | "COMPLETED" | "IDLE" | "ERROR";

export type DisplaySource = "manual_preview" | "codex";

export interface DisplaySnapshot {
  state: DisplayState;
  source: DisplaySource;
}

export const DISPLAY_STATES: DisplayState[] = ["WAITING", "WORKING", "COMPLETED", "IDLE"];

export const DISPLAY_META: Record<DisplayState, { label: string; detail: string; color: string }> = {
  WAITING: {
    label: "Waiting",
    detail: "Human attention may be required",
    color: "#ef4444",
  },
  WORKING: {
    label: "Working",
    detail: "The agent is actively processing",
    color: "#facc15",
  },
  COMPLETED: {
    label: "Completed",
    detail: "A tracked task has completed",
    color: "#4ade80",
  },
  IDLE: {
    label: "Idle",
    detail: "No active work or attention request",
    color: "#94a3b8",
  },
  ERROR: {
    label: "Error",
    detail: "An agent or integration reported an error",
    color: "#ef4444",
  },
};
