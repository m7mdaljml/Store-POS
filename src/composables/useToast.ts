import { reactive } from "vue";

export type ToastType = "success" | "error" | "info";

export interface Toast {
  id: number;
  type: ToastType;
  message: string;
}

const state = reactive<{ items: Toast[] }>({ items: [] });
let nextId = 1;

function show(type: ToastType, message: string, timeoutMs = 4500): number {
  const id = nextId++;
  state.items.push({ id, type, message });
  if (state.items.length > 6) state.items.shift();
  window.setTimeout(() => dismiss(id), timeoutMs);
  return id;
}

function dismiss(id: number): void {
  const index = state.items.findIndex((t) => t.id === id);
  if (index !== -1) state.items.splice(index, 1);
}

/** Global toast notifications (F9.3). Shared singleton via Vue reactivity. */
export function useToast() {
  return {
    items: state.items,
    dismiss,
    success: (msg: string) => show("success", msg),
    error: (msg: string) => show("error", msg, 7000),
    info: (msg: string) => show("info", msg),
  };
}
