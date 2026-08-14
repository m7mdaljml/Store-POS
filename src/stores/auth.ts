import { computed, ref } from "vue";
import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import type { AuthUser } from "../types";

export const useAuthStore = defineStore("auth", () => {
  const user = ref<AuthUser | null>(null);

  const isAuthenticated = computed(() => user.value !== null);

  async function login(username: string, password: string): Promise<void> {
    user.value = await invoke<AuthUser>("login", { username, password });
  }

  function logout() {
    user.value = null;
    invoke("logout").catch(() => {});
  }

  function can(code: string): boolean {
    return user.value?.permissions.includes(code) ?? false;
  }

  return { user, isAuthenticated, login, logout, can };
});
