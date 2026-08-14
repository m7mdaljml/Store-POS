import { computed, ref } from "vue";
import { defineStore } from "pinia";
import type { AuthUser } from "../types";

export const useAuthStore = defineStore("auth", () => {
  const user = ref<AuthUser | null>(null);

  const isAuthenticated = computed(() => user.value !== null);

  function login(u: AuthUser) {
    user.value = u;
  }

  function logout() {
    user.value = null;
  }

  return { user, isAuthenticated, login, logout };
});
