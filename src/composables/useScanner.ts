import { onBeforeUnmount } from "vue";

export interface ScannerOptions {
  /** Called when a complete scan burst ends with Enter. */
  onScan: (code: string) => void;
  /**
   * Max gap in ms between keystrokes for them to count as a single scan
   * burst (USB scanners send characters ~5–30 ms apart). Default 50.
   */
  burstMs?: number;
  /** Minimum burst length to treat as a barcode read (avoids single-char false positives). Default 4. */
  minLength?: number;
  /** Safety cap on buffer length. Default 64. */
  maxLength?: number;
}

/**
 * Detects barcode scanner input from rapid keystroke bursts that end with
 * Enter. Listens in the capture phase so a completed scan can consume the
 * Enter key before any focused input handles it.
 */
export function useScanner(options: ScannerOptions) {
  const { onScan } = options;
  const burstMs = options.burstMs ?? 50;
  const minLength = options.minLength ?? 4;
  const maxLength = options.maxLength ?? 64;

  let buffer = "";
  let lastCharAt = 0;

  function onKeydown(e: KeyboardEvent) {
    if (e.isComposing || e.repeat) return;
    if (e.altKey || e.ctrlKey || e.metaKey) return;

    if (e.key.length === 1) {
      const now = Date.now();
      if (lastCharAt && now - lastCharAt > burstMs) buffer = "";
      lastCharAt = now;
      buffer = (buffer + e.key).slice(-maxLength);
      return;
    }

    if (e.key === "Enter") {
      const code = buffer;
      buffer = "";
      lastCharAt = 0;
      if (code.length >= minLength) {
        e.preventDefault();
        e.stopPropagation();
        onScan(code);
      }
    }
  }

  window.addEventListener("keydown", onKeydown, true);
  onBeforeUnmount(() => window.removeEventListener("keydown", onKeydown, true));
}
