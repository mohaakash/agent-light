export type SoundPreset = "gentle_chime" | "alert_knock" | "modern_ping";

export interface AudioAlertSettings {
  sound_enabled: boolean;
  sound_volume: number; // 0 - 100
  sound_on_attention: boolean;
  sound_on_completion: boolean;
  sound_preset: SoundPreset;
  notifications_enabled: boolean;
  do_not_disturb: boolean;
  do_not_disturb_until: string | null;
  suppress_when_focused: boolean;
}

/**
 * Whether alerts are currently silenced by do-not-disturb, either because it
 * was enabled manually or because a timed window is still running.
 */
export function isDoNotDisturbActive(settings: AudioAlertSettings, now = Date.now()): boolean {
  if (settings.do_not_disturb) return true;
  if (!settings.do_not_disturb_until) return false;
  const until = Date.parse(settings.do_not_disturb_until);
  return Number.isFinite(until) && until > now;
}

let audioCtx: AudioContext | null = null;

function getAudioContext(): AudioContext | null {
  if (typeof window === "undefined") return null;
  const AudioContextClass = window.AudioContext || (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext;
  if (!AudioContextClass) return null;
  if (!audioCtx || audioCtx.state === "closed") {
    audioCtx = new AudioContextClass();
  }
  if (audioCtx.state === "suspended") {
    void audioCtx.resume().catch(() => undefined);
  }
  return audioCtx;
}

function playGentleChime(ctx: AudioContext, gainNode: GainNode, startTime: number) {
  // Harmonic chime: D5 (587.33 Hz) and A5 (880 Hz)
  const notes = [587.33, 880];
  notes.forEach((freq, idx) => {
    const osc = ctx.createOscillator();
    const noteGain = ctx.createGain();

    osc.type = "sine";
    osc.frequency.setValueAtTime(freq, startTime + idx * 0.04);

    noteGain.gain.setValueAtTime(0, startTime + idx * 0.04);
    noteGain.gain.linearRampToValueAtTime(0.5, startTime + idx * 0.04 + 0.02);
    noteGain.gain.exponentialRampToValueAtTime(0.001, startTime + idx * 0.04 + 0.65);

    osc.connect(noteGain);
    noteGain.connect(gainNode);

    osc.start(startTime + idx * 0.04);
    osc.stop(startTime + idx * 0.04 + 0.7);
  });
}

function playAlertKnock(ctx: AudioContext, gainNode: GainNode, startTime: number) {
  // Two wooden percussive taps
  [0, 0.12].forEach((offset) => {
    const osc = ctx.createOscillator();
    const noteGain = ctx.createGain();

    osc.type = "triangle";
    osc.frequency.setValueAtTime(180, startTime + offset);
    osc.frequency.exponentialRampToValueAtTime(60, startTime + offset + 0.07);

    noteGain.gain.setValueAtTime(0.7, startTime + offset);
    noteGain.gain.exponentialRampToValueAtTime(0.001, startTime + offset + 0.08);

    osc.connect(noteGain);
    noteGain.connect(gainNode);

    osc.start(startTime + offset);
    osc.stop(startTime + offset + 0.09);
  });
}

function playModernPing(ctx: AudioContext, gainNode: GainNode, startTime: number) {
  // Modern glass crystal ping: E6 (1318.5 Hz) and B6 (1975.5 Hz)
  const osc1 = ctx.createOscillator();
  const osc2 = ctx.createOscillator();
  const noteGain = ctx.createGain();

  osc1.type = "sine";
  osc1.frequency.setValueAtTime(1318.51, startTime);

  osc2.type = "sine";
  osc2.frequency.setValueAtTime(1975.53, startTime);

  noteGain.gain.setValueAtTime(0.6, startTime);
  noteGain.gain.exponentialRampToValueAtTime(0.001, startTime + 0.5);

  osc1.connect(noteGain);
  osc2.connect(noteGain);
  noteGain.connect(gainNode);

  osc1.start(startTime);
  osc2.start(startTime);
  osc1.stop(startTime + 0.55);
  osc2.stop(startTime + 0.55);
}

function playCompletionTriad(ctx: AudioContext, gainNode: GainNode, startTime: number) {
  // Ascending major chord: C5 (523Hz), E5 (659Hz), G5 (784Hz)
  const notes = [523.25, 659.25, 783.99];
  notes.forEach((freq, idx) => {
    const osc = ctx.createOscillator();
    const noteGain = ctx.createGain();

    osc.type = "sine";
    osc.frequency.setValueAtTime(freq, startTime + idx * 0.08);

    noteGain.gain.setValueAtTime(0, startTime + idx * 0.08);
    noteGain.gain.linearRampToValueAtTime(0.4, startTime + idx * 0.08 + 0.02);
    noteGain.gain.exponentialRampToValueAtTime(0.001, startTime + idx * 0.08 + 0.55);

    osc.connect(noteGain);
    noteGain.connect(gainNode);

    osc.start(startTime + idx * 0.08);
    osc.stop(startTime + idx * 0.08 + 0.6);
  });
}

export function unlockAudioContext() {
  const ctx = getAudioContext();
  if (ctx?.state === "suspended") {
    void ctx.resume().catch(() => undefined);
  }
}

export function previewSound(preset: SoundPreset, volume: number) {
  const ctx = getAudioContext();
  if (!ctx) return;

  const masterGain = ctx.createGain();
  const normalizedVol = Math.max(0, Math.min(100, volume)) / 100;
  masterGain.gain.setValueAtTime(normalizedVol * 0.35, ctx.currentTime);
  masterGain.connect(ctx.destination);

  const startTime = ctx.currentTime;
  switch (preset) {
    case "alert_knock":
      playAlertKnock(ctx, masterGain, startTime);
      break;
    case "modern_ping":
      playModernPing(ctx, masterGain, startTime);
      break;
    case "gentle_chime":
    default:
      playGentleChime(ctx, masterGain, startTime);
      break;
  }
}

export function playAudioAlert(
  type: "attention" | "error" | "completion",
  settings: AudioAlertSettings
) {
  if (isDoNotDisturbActive(settings)) return;
  if (!settings.sound_enabled || settings.sound_volume <= 0) return;
  if ((type === "attention" || type === "error") && !settings.sound_on_attention) return;
  if (type === "completion" && !settings.sound_on_completion) return;

  const ctx = getAudioContext();
  if (!ctx) return;

  const masterGain = ctx.createGain();
  const normalizedVol = Math.max(0, Math.min(100, settings.sound_volume)) / 100;
  masterGain.gain.setValueAtTime(normalizedVol * 0.35, ctx.currentTime);
  masterGain.connect(ctx.destination);

  const startTime = ctx.currentTime;
  if (type === "completion") {
    playCompletionTriad(ctx, masterGain, startTime);
  } else if (type === "error") {
    playAlertKnock(ctx, masterGain, startTime);
  } else {
    switch (settings.sound_preset) {
      case "alert_knock":
        playAlertKnock(ctx, masterGain, startTime);
        break;
      case "modern_ping":
        playModernPing(ctx, masterGain, startTime);
        break;
      case "gentle_chime":
      default:
        playGentleChime(ctx, masterGain, startTime);
        break;
    }
  }
}
