<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useSettingsStore } from "../../stores/settings";
import type { SaleSession } from "../../types";

const settings = useSettingsStore();

const sessions = ref<SaleSession[]>([]);
const loading = ref(false);
const error = ref("");
const filter = ref("");

function fmt(n: number): string {
  if (!settings.currency) return n.toFixed(2);
  return new Intl.NumberFormat(undefined, {
    style: "currency",
    currency: settings.currency,
    currencyDisplay: "narrowSymbol",
  }).format(n);
}

function dateLabel(d: string): string {
  const date = new Date(d + (d.includes("T") ? "" : "Z"));
  if (isNaN(date.getTime())) return d;
  return date.toLocaleString();
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

<template>
  <div class="page-container">
    <div class="page-heading d-flex flex-wrap align-items-center gap-3">
      <div>
        <h2 class="mb-0">Register sessions</h2>
        <span class="text-muted small">Open and closed cash register shifts</span>
      </div>
      <div class="ms-auto d-flex align-items-center gap-2">
        <div class="btn-group btn-group-sm" role="group" aria-label="Filter by status">
          <button
            type="button"
            class="btn btn-outline-secondary"
            :class="{ active: filter === '' }"
            @click="filter = ''"
          >
            All
          </button>
          <button
            type="button"
            class="btn btn-outline-secondary"
            :class="{ active: filter === 'open' }"
            @click="filter = 'open'"
          >
            Open
          </button>
          <button
            type="button"
            class="btn btn-outline-secondary"
            :class="{ active: filter === 'closed' }"
            @click="filter = 'closed'"
          >
            Closed
          </button>
        </div>
        <button class="btn btn-sm btn-outline-primary" type="button" @click="load">
          <i class="bi bi-arrow-clockwise me-1"></i>Refresh
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
              <th>Opened</th>
              <th>Closed</th>
              <th>Cashier</th>
              <th class="text-end">Opening</th>
              <th class="text-end">Sales</th>
              <th class="text-end">Total</th>
              <th class="text-end">Expected</th>
              <th class="text-end">Counted</th>
              <th class="text-end">Variance</th>
              <th>Status</th>
            </tr>
          </thead>
          <tbody>
            <tr v-if="loading">
              <td colspan="11" class="text-center py-4 text-muted">
                <span class="spinner-border spinner-border-sm me-2" role="status"></span>Loading…
              </td>
            </tr>
            <tr v-else-if="!filteredSessions.length">
              <td colspan="11" class="text-center py-4 text-muted">No sessions found</td>
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
                  {{ s.status }}
                </span>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  </div>
</template>
