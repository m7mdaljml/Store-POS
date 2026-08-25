import { computed, ref } from "vue";
import { defineStore } from "pinia";
import { execute, select } from "../lib/db";
import type { SettingsMap } from "../types";

/** Allowed rows-per-page values for list screens. */
export const PAGE_SIZE_OPTIONS = [5, 10, 20, 100];
const DEFAULT_PAGE_SIZE = 20;

export const useSettingsStore = defineStore("settings", () => {
  const values = ref<SettingsMap>({});
  const loaded = ref(false);

  const storeName = computed(() => values.value["store_name"] ?? "");
  const storeAddress = computed(() => values.value["store_address"] ?? "");
  const storePhone = computed(() => values.value["store_phone"] ?? "");
  const taxId = computed(() => values.value["tax_id"] ?? "");
  const currency = computed(() => values.value["currency"] ?? "");
  const discountThreshold = computed(() => {
    const parsed = Number(values.value["discount_threshold"]);
    return isNaN(parsed) || parsed < 0 ? 10 : parsed;
  });
  /** Store logo as data URL (empty string when unset). */
  const storeLogo = computed(() => values.value["store_logo"] ?? "");
  const receiptHeader = computed(() => values.value["receipt_header"] ?? "");
  const receiptFooter = computed(() => values.value["receipt_footer"] ?? "");
  const receiptLogoPos = computed(
    () => (values.value["receipt_logo_pos"] as "top" | "bottom") ?? "top",
  );
  const receiptFormat = computed(
    () => (values.value["receipt_format"] as "thermal" | "a4") ?? "thermal",
  );
  const soundEnabled = computed(() => values.value["sound_enabled"] === "1");
  /** Rows per page shown in list screens (user preference). */
  const pageSize = computed(() => {
    const n = Number(values.value["page_size"]);
    return PAGE_SIZE_OPTIONS.includes(n) ? n : DEFAULT_PAGE_SIZE;
  });

  async function load() {
    const rows = await select<{ key: string; value: string }>(
      "SELECT key, value FROM settings",
    );
    const map: SettingsMap = {};
    for (const row of rows) map[row.key] = row.value;
    values.value = map;
    loaded.value = true;
  }

  /**
   * Persists one setting and keeps the local map in sync so every consumer
   * reacts immediately without reloading.
   */
  async function setValue(key: string, value: string) {
    await execute(
      "INSERT INTO settings (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
      [key, value],
    ).catch((e: unknown) => {
      console.error(`Failed saving setting "${key}"`, e);
      throw e;
    });
    values.value[key] = value;
  }

  return {
    values,
    loaded,
    storeName,
    storeAddress,
    storePhone,
    taxId,
    currency,
    discountThreshold,
    storeLogo,
    receiptHeader,
    receiptFooter,
    receiptLogoPos,
    receiptFormat,
    soundEnabled,
    pageSize,
    load,
    setValue,
  };
});
