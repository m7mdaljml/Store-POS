<template>
  <div class="page-container">
    <div class="page-heading d-flex flex-wrap align-items-center gap-3">
      <div>
        <h2 class="mb-0">{{ t("sessions.title") }}</h2>
        <span class="text-muted small">{{ t("sessions.subtitle") }}</span>
      </div>
      <div class="ms-auto d-flex align-items-center gap-2">
        <div class="btn-group btn-group-sm" role="group" :aria-label="t('sales.filterStatusAria')">
          <button
            type="button"
            class="btn btn-outline-secondary"
            :class="{ active: filter === '' }"
            @click="filter = ''"
          >
            {{ t("common.all") }}
          </button>
          <button
            type="button"
            class="btn btn-outline-secondary"
            :class="{ active: filter === 'open' }"
            @click="filter = 'open'"
          >
            {{ t("sessions.open") }}
          </button>
          <button
            type="button"
            class="btn btn-outline-secondary"
            :class="{ active: filter === 'closed' }"
            @click="filter = 'closed'"
          >
            {{ t("sessions.closed") }}
          </button>
        </div>
        <button class="btn btn-sm btn-outline-primary" type="button" @click="load">
          <i class="bi bi-arrow-clockwise me-1"></i>{{ t("common.refresh") }}
        </button>
      </div>
    </div>

    <div v-if="error" class="alert alert-warning py-1 px-2 small" role="alert">
      <i class="bi bi-exclamation-triangle me-1"></i>{{ error }}
    </div>

    <div class="card">
      <div class="table-responsive">
        <table class="table table-hover align-middle mb-0">
          <thead>
            <tr>
              <th>#</th>
              <th>{{ t("sessions.opened") }}</th>
              <th>{{ t("sessions.closedCol") }}</th>
              <th>{{ t("common.cashier") }}</th>
              <th class="text-end">{{ t("sessions.opening") }}</th>
              <th class="text-end">{{ t("sessions.sales") }}</th>
              <th class="text-end">{{ t("common.total") }}</th>
              <th class="text-end">{{ t("sessions.expected") }}</th>
              <th class="text-end">{{ t("sessions.counted") }}</th>
              <th class="text-end">{{ t("sessions.variance") }}</th>
              <th>{{ t("common.status") }}</th>
            </tr>
          </thead>
          <tbody>
            <tr v-if="loading">
              <td colspan="11" class="text-center py-4 text-muted">
                <span class="spinner-border spinner-border-sm me-2" role="status"></span>{{ t("common.loading") }}
              </td>
            </tr>
            <tr v-else-if="!filteredSessions.length">
              <td colspan="11" class="text-center py-4 text-muted">{{ t("sessions.noSessions") }}</td>
            </tr>
            <tr v-for="s in filteredSessions" :key="s.id">
              <td class="text-muted">{{ s.id }}</td>
              <td class="text-muted">{{ dateLabel(s.openedAt) }}</td>
              <td class="text-muted">{{ s.closedAt ? dateLabel(s.closedAt) : "—" }}</td>
              <td>{{ s.userName || "—" }}</td>
              <td class="text-end">{{ fmt(s.openingCash) }}</td>
              <td class="text-end">{{ s.salesCount }}</td>
              <td class="text-end">{{ fmt(s.salesTotal) }}</td>
              <td class="text-end">{{ s.expectedCash != null ? fmt(s.expectedCash) : "—" }}</td>
              <td class="text-end">{{ s.closingCash != null ? fmt(s.closingCash) : "—" }}</td>
              <td
                class="text-end fw-semibold"
                :class="s.status === 'closed' && (s.variance ?? 0) < -0.005
                  ? 'text-danger'
                  : s.status === 'closed' && (s.variance ?? 0) > 0.005
                    ? 'text-warning'
                    : 'text-success'"
              >
                {{ s.variance != null ? fmt(s.variance) : "—" }}
              </td>
              <td>
                <span
                  class="badge"
                  :class="s.status === 'open' ? 'text-bg-primary' : 'text-bg-secondary'"
                >
                  {{ s.status === "open" ? t("sessions.open") : t("sessions.closed") }}
                </span>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useI18n } from "vue-i18n";
import { useSettingsStore } from "../../stores/settings";
import type { SaleSession } from "../../types";

const settings = useSettingsStore();
const { t, locale } = useI18n();

const sessions = ref<SaleSession[]>([]);
const loading = ref(false);
const error = ref("");
const filter = ref("");

function fmt(n: number): string {
  if (!settings.currency) return n.toFixed(2);
  return new Intl.NumberFormat(locale.value, {
    style: "currency",
    currency: settings.currency,
    currencyDisplay: "narrowSymbol",
  }).format(n);
}

function dateLabel(d: string): string {
  const date = new Date(d + (d.includes("T") ? "" : "Z"));
  if (isNaN(date.getTime())) return d;
  return date.toLocaleString(locale.value);
}

const filteredSessions = computed(() =>
  filter.value ? sessions.value.filter((s) => s.status === filter.value) : sessions.value
);

async function load() {
  loading.value = true;
  error.value = "";
  try {
    sessions.value = await invoke<SaleSession[]>("list_sessions", {
      input: { status: null, limit: 200 },
    });
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

onMounted(load);
</script>
