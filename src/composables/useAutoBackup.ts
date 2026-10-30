import { invoke } from "@tauri-apps/api/core";
import { useSettingsStore } from "../stores/settings";
import { useAuthStore } from "../stores/auth";

/** How often to check whether a backup is due. */
const CHECK_INTERVAL_MS = 10 * 60 * 1000;

function thresholdMs(freq: string): number {
  return freq === "weekly" ? 7 * 24 * 60 * 60 * 1000 : 24 * 60 * 60 * 1000;
}

/** SQLite's `datetime('now')` is UTC "YYYY-MM-DD HH:MM:SS". */
function parseDbTime(raw: string | null | undefined): number {
  if (!raw) return 0;
  const normalized = raw.includes("T") ? raw : raw.replace(" ", "T");
  const ms = Date.parse(normalized.endsWith("Z") ? normalized : normalized + "Z");
  return isNaN(ms) ? 0 : ms;
}

let timer: ReturnType<typeof setInterval> | undefined;

async function checkAndBackup(): Promise<void> {
  const auth = useAuthStore();
  const settings = useSettingsStore();
  if (!auth.isAuthenticated || !settings.loaded) return;
  // Backups are an admin/settings operation; skip for users without the right.
  if (auth.role !== "Admin" && !auth.can("settings.manage")) return;
  if (settings.values["backup_auto"] !== "1") return;

  const freq = settings.values["backup_freq"] === "weekly" ? "weekly" : "daily";
  let lastMs = 0;
  try {
    const last = await invoke<string | null>("last_backup_time");
    lastMs = parseDbTime(last);
  } catch {
    lastMs = 0; // no backups table yet → definitely due
  }
  if (Date.now() - lastMs < thresholdMs(freq)) return;

  await invoke("create_backup", {
    dir: settings.values["backup_dir"] || null,
    kind: "auto",
  }).catch((e: unknown) => console.error("Auto-backup failed:", e));
}

/**
 * Starts the F8.3 scheduled-backup watchdog. Safe to call multiple times;
 * the interval runs even before login and skips work until eligible.
 */
export function startAutoBackup(): void {
  if (timer) return;
  void checkAndBackup();
  timer = setInterval(() => void checkAndBackup(), CHECK_INTERVAL_MS);
}
