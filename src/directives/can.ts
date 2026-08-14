import type { Directive } from "vue";
import { useAuthStore } from "../stores/auth";

export const can: Directive<HTMLElement, string | string[]> = {
  mounted(el, binding) {
    const auth = useAuthStore();
    const codes = Array.isArray(binding.value) ? binding.value : [binding.value];
    el.style.display = codes.some((code) => auth.can(code)) ? "" : "none";
  },
  updated(el, binding) {
    const auth = useAuthStore();
    const codes = Array.isArray(binding.value) ? binding.value : [binding.value];
    el.style.display = codes.some((code) => auth.can(code)) ? "" : "none";
  },
};
