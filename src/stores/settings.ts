import { computed, ref } from "vue";
import { defineStore } from "pinia";
import { select } from "../lib/db";
import type { SettingsMap } from "../types";

export const useSettingsStore = defineStore("settings", () => {
  const values = ref<SettingsMap>({});
  const loaded = ref(false);

  const storeName = computed(() => values.value["store_name"] ?? "");
  const taxId = computed(() => values.value["tax_id"] ?? "");
  const currency = computed(() => values.value["currency"] ?? "");
  const discountThreshold = computed(() => {
    const parsed = Number(values.value["discount_threshold"]);
    return isNaN(parsed) || parsed < 0 ? 10 : parsed;
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

  return {
    values,
    loaded,
    storeName,
    taxId,
    currency,
    discountThreshold,
    load,
  };
});
