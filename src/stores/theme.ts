import { computed, ref } from "vue";
import { defineStore } from "pinia";

export type Theme = "dark" | "light";

const STORAGE_KEY = "pos_theme";

function applyTheme(theme: Theme) {
  document.documentElement.setAttribute("data-bs-theme", theme);
}

export const useThemeStore = defineStore("theme", () => {
  const theme = ref<Theme>((localStorage.getItem(STORAGE_KEY) as Theme) || "dark");
  applyTheme(theme.value);

  const isDark = computed(() => theme.value === "dark");

  function toggle() {
    set(isDark.value ? "light" : "dark");
  }

  function set(next: Theme) {
    theme.value = next;
    localStorage.setItem(STORAGE_KEY, next);
    applyTheme(next);
  }

  return { theme, isDark, toggle, set };
});
