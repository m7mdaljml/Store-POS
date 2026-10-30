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

function tone(
  ac: AudioContext,
  type: OscillatorType,
  freq: number,
  startAt: number,
  dur: number,
  vol: number,
): void {
  const osc = ac.createOscillator();
  const gain = ac.createGain();
  osc.type = type;
  osc.frequency.setValueAtTime(freq, startAt);
  gain.gain.setValueAtTime(vol, startAt);
  gain.gain.exponentialRampToValueAtTime(0.0001, startAt + dur);
  osc.connect(gain).connect(ac.destination);
  osc.start(startAt);
  osc.stop(startAt + dur);
}

/** Short beep used for cart/sale/scan feedback (no asset needed). */
export function playBeep(kind: "click" | "success" | "error" = "click"): void {
  if (!useSettingsStore().soundEnabled) return;
  const ac = audioContext();
  if (!ac) return;

  const now = ac.currentTime;
  if (kind === "error") {
    // Two descending low buzzes - clearly distinct from the click/success tones.
    tone(ac, "sawtooth", 330, now, 0.16, 0.07);
    tone(ac, "sawtooth", 180, now + 0.18, 0.22, 0.07);
    return;
  }

  const [freq, dur, vol] =
    kind === "success" ? [1046, 0.18, 0.05] : [880, 0.07, 0.035];
  tone(ac, "square", freq, now, dur, vol);
}
