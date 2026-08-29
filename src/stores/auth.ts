import { computed, ref } from "vue";
import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import type { AuthUser } from "../types";

const SESSION_KEY = "pos_session";
const SESSION_DURATION_MS = 8 * 60 * 60 * 1000;

interface StoredSession {
  user: AuthUser;
  expiresAt: number;
}

export const useAuthStore = defineStore("auth", () => {
  const user = ref<AuthUser | null>(null);
  const expiresAt = ref(0);
  let expiryTimer: number | undefined;

  const isAuthenticated = computed(() => user.value !== null);
  const role = computed(() => user.value?.roleName ?? null);
  const permissions = computed(() => user.value?.permissions ?? []);
  const mustChangePassword = computed(() => user.value?.mustChangePassword ?? false);

  function persist() {
    if (!user.value) return;
    localStorage.setItem(
      SESSION_KEY,
      JSON.stringify({ user: user.value, expiresAt: expiresAt.value } satisfies StoredSession)
    );
  }

  function scheduleExpiry() {
    if (expiryTimer) window.clearTimeout(expiryTimer);
    const remaining = expiresAt.value - Date.now();
    if (remaining <= 0) {
      logout();
      return;
    }
    expiryTimer = window.setTimeout(logout, remaining);
  }

  function clearSession() {
    if (expiryTimer) window.clearTimeout(expiryTimer);
    expiryTimer = undefined;
    user.value = null;
    expiresAt.value = 0;
    localStorage.removeItem(SESSION_KEY);
  }

  function hydrate() {
    const raw = localStorage.getItem(SESSION_KEY);
    if (!raw) return;
    try {
      const stored = JSON.parse(raw) as StoredSession;
      if (stored.user && stored.expiresAt > Date.now()) {
        user.value = stored.user;
        expiresAt.value = stored.expiresAt;
        scheduleExpiry();
      } else {
        clearSession();
      }
    } catch {
      clearSession();
    }
  }

  async function verifySession(): Promise<void> {
    if (!user.value) return;
    const valid = await invoke<boolean>("verify_session", { userId: user.value.id });
    if (!valid) logout();
  }

  async function login(username: string, password: string): Promise<void> {
    user.value = await invoke<AuthUser>("login", { username, password });
    expiresAt.value = Date.now() + SESSION_DURATION_MS;
    persist();
    scheduleExpiry();
  }

  function logout() {
    clearSession();
    invoke("logout").catch(() => {});
  }

  async function setOwnPassword(newPassword: string): Promise<void> {
    if (!user.value) throw new Error("Not authenticated");
    await invoke("set_own_password", {
      userId: user.value.id,
      newPassword,
    });
    user.value = { ...user.value, mustChangePassword: false };
    persist();
  }

  function can(code: string): boolean {
    return user.value?.permissions.includes(code) ?? false;
  }

  return { user, role, permissions, expiresAt, isAuthenticated, mustChangePassword, login, logout, hydrate, verifySession, setOwnPassword, can };
});
