import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { i18n } from "../i18n";

/**
 * Base-currency display helpers. Money amounts are always rendered with the
 * base currency symbol placed AFTER the number (e.g. "12.50 JD"), regardless
 * of the active UI locale.
 */

/** Reactive base currency code (ISO, e.g. "JOD"; empty until loaded). */
export const baseCurrencyCode = ref("");

/** Reactive base currency symbol (e.g. "JD"; falls back to the code). */
export const baseCurrencySymbol = ref("");

/**
 * Loads the base currency code/symbol from the backend, which falls back to
 * the ISO code stored under the "currency" settings key.
 */
export async function loadBaseCurrencySymbol(): Promise<void> {
  try {
    const info = await invoke<{ code: string; symbol: string }>(
      "get_base_currency",
    );
    baseCurrencyCode.value = (info.code ?? "").trim();
    baseCurrencySymbol.value = (info.symbol ?? "").trim() || baseCurrencyCode.value;
  } catch {
    // Leave empty: formatting degrades gracefully to plain numbers.
  }
}

let loadStarted = false;

/**
 * Kicks off a one-time background load of the base currency info. If the load
 * produced nothing (e.g. it ran before sign-in), it is allowed to retry so the
 * symbol appears once a session exists.
 */
export function ensureBaseCurrencySymbol(): void {
  if (loadStarted) return;
  loadStarted = true;
  void loadBaseCurrencySymbol().then(() => {
    if (!baseCurrencySymbol.value) loadStarted = false;
  });
}

const formatters = new Map<string, Intl.NumberFormat>();

/**
 * Number-only formatter (no currency style) that keeps the fraction digits
 * the locale would normally use for the given currency code.
 */
function numberFormatter(locale: string, code: string): Intl.NumberFormat {
  const key = `${locale}|${code}`;
  let f = formatters.get(key);
  if (!f) {
    let min = 2;
    let max = 2;
    try {
      const resolved = new Intl.NumberFormat(locale, {
        style: "currency",
        currency: code,
      }).resolvedOptions();
      min = resolved.minimumFractionDigits ?? 2;
      max = resolved.maximumFractionDigits ?? 2;
    } catch {
      // Unknown/empty code: keep plain 2-decimal formatting.
    }
    f = new Intl.NumberFormat(locale, {
      minimumFractionDigits: min,
      maximumFractionDigits: max,
    });
    formatters.set(key, f);
  }
  return f;
}

/**
 * Formats `n` as "<number> <symbol>" using the active UI locale for digits
 * and grouping. Degrades to plain numbers when no base currency is known.
 */
export function formatMoney(n: number): string {
  ensureBaseCurrencySymbol();
  const value = Number.isFinite(n) ? n : 0;
  const num =
    Math.round(Math.abs(value) * 100) === 0
      ? "0"
      : numberFormatter(
          i18n.global.locale.value,
          baseCurrencyCode.value,
        ).format(value);
  const symbol = baseCurrencySymbol.value;
  return symbol ? `${num} ${symbol}` : num;
}

/** Suffix (" JD") appended after bare numbers on printed receipts. */
export function currencySuffix(): string {
  ensureBaseCurrencySymbol();
  const s = baseCurrencySymbol.value;
  return s ? ` ${s}` : "";
}
