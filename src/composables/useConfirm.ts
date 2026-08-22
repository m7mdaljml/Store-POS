import { reactive } from "vue";
import { i18n } from "../i18n";

export interface ConfirmOptions {
  message: string;
  title?: string;
  confirmLabel?: string;
  cancelLabel?: string;
  danger?: boolean;
}

interface ConfirmState {
  show: boolean;
  title: string;
  message: string;
  confirmLabel: string;
  cancelLabel: string;
  danger: boolean;
}

const state = reactive<ConfirmState>({
  show: false,
  title: "",
  message: "",
  confirmLabel: "",
  cancelLabel: "",
  danger: false,
});

let resolver: ((ok: boolean) => void) | null = null;

function settle(ok: boolean) {
  if (!state.show) return;
  state.show = false;
  const resolve = resolver;
  resolver = null;
  resolve?.(ok);
}

/**
 * Promise-based confirmation dialog that replaces `window.confirm`, which
 * Tauri v2 webviews do not support (it surfaces as a "dialog.confirm not
 * allowed / command not found" error). Mount <ConfirmHost /> once near the
 * app root and `await confirmDialog(...)` wherever a decision is needed.
 */
function confirmDialog(options: ConfirmOptions): Promise<boolean> {
  return new Promise((resolve) => {
    settle(false);
    resolver = resolve;
    state.title = options.title ?? i18n.global.t("common.areYouSure");
    state.message = options.message;
    state.confirmLabel =
      options.confirmLabel ?? i18n.global.t("common.confirm");
    state.cancelLabel = options.cancelLabel ?? i18n.global.t("common.cancel");
    state.danger = options.danger ?? true;
    state.show = true;
  });
}

/** Shared singleton consumed by ConfirmHost.vue and all call sites. */
export function useConfirm() {
  return { confirmDialog, settle, state };
}
