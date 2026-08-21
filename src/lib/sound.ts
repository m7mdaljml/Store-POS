import { useSettingsStore } from "../stores/settings";

let ctx: AudioContext | null = null;

function audioContext(): AudioContext | null {
  try {
    ctx ??= new AudioContext();
    if (ctx.state === "suspended") void ctx.resume();
    return ctx;
  } catch {
    return null;
  }
}

/** Short two-tone beep used for cart/sale feedback (no asset needed). */
export function playBeep(kind: "click" | "success" = "click"): void {
  if (!useSettingsStore().soundEnabled) return;
  const ac = audioContext();
  if (!ac) return;

  const osc = ac.createOscillator();
  const gain = ac.createGain();
  osc.type = "square";
  const now = ac.currentTime;
  const [freq, dur, vol] =
    kind === "success" ? [1046, 0.18, 0.05] : [880, 0.07, 0.035];
  osc.frequency.setValueAtTime(freq, now);
  gain.gain.setValueAtTime(vol, now);
  gain.gain.exponentialRampToValueAtTime(0.0001, now + dur);
  osc.connect(gain).connect(ac.destination);
  osc.start(now);
  osc.stop(now + dur);
}
