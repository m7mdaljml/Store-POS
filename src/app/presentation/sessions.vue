<template>
  <div>
    <div class="d-flex align-items-center justify-content-between mb-3">
      <h1 class="h4 mb-0">{{ t("sessions.title") }}</h1>
      <div class="d-flex align-items-center gap-2">
        <div
          class="btn-group btn-group-sm"
          role="group"
          :aria-label="t('sales.filterStatusAria')"
        >
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
        <button
          class="btn btn-sm btn-outline-primary"
          type="button"
          @click="reload"
        >
          <i class="bi bi-arrow-clockwise mx-1"></i>{{ t("common.refresh") }}
        </button>
      </div>
    </div>



    <div class="card">
      <div class="p-2 border-bottom">
        <input
          v-model="search"
          class="form-control form-control-sm"
          type="search"
          :placeholder="t('sessions.searchPlaceholder')"
        />
      </div>
      <div class="table-responsive">
        <table class="table table-hover align-middle mb-0">
          <thead v-if="sessions.length">
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
          <tbody v-if="loading">
            <tr>
              <td colspan="11" class="text-center py-4 text-muted">
                <span
                  class="spinner-border spinner-border-sm mx-2"
                  role="status"
                ></span
                >{{ t("common.loading") }}
              </td>
            </tr>
          </tbody>
          <tbody v-else-if="!sessions.length">
            <tr>
              <td colspan="11" class="p-0 border-0">
                <EmptyState
                  :image="emptySessions"
                  :message="t('sessions.noSessions')"
                />
              </td>
            </tr>
          </tbody>
          <tbody v-for="s in sessions" :key="s.id">
            <tr>
              <td class="text-muted">{{ s.id }}</td>
              <td class="text-muted">{{ dateLabel(s.openedAt) }}</td>
              <td class="text-muted">
                {{ s.closedAt ? dateLabel(s.closedAt) : "—" }}
              </td>
              <td>{{ s.userName || "—" }}</td>
              <td class="text-end">{{ fmt(s.openingCash) }}</td>
              <td class="text-end">{{ s.salesCount }}</td>
              <td class="text-end">{{ fmt(s.salesTotal) }}</td>
              <td class="text-end">
                {{ s.expectedCash != null ? fmt(s.expectedCash) : "—" }}
              </td>
              <td class="text-end">
                {{ s.closingCash != null ? fmt(s.closingCash) : "—" }}
              </td>
              <td
                class="text-end fw-semibold"
                :class="
                  s.status === 'closed' && (s.variance ?? 0) < -0.005
                    ? 'text-danger'
                    : s.status === 'closed' && (s.variance ?? 0) > 0.005
                      ? 'text-warning'
                      : 'text-success'
                "
              >
                {{ s.variance != null ? fmt(s.variance) : "—" }}
              </td>
              <td>
                <span
                  class="badge"
                  :class="
                    s.status === 'open'
                      ? 'text-bg-primary'
                      : 'text-bg-secondary'
                  "
                >
                  {{
                    s.status === "open"
                      ? t("sessions.open")
                      : t("sessions.closed")
                  }}
                </span>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
      <div
        v-if="hasMore && !loading"
        class="text-center border-top py-2"
      >
        <button
          class="btn btn-sm btn-outline-secondary"
          type="button"
          :disabled="loadingMore"
          @click="loadMore"
        >
          <span
            v-if="loadingMore"
            class="spinner-border spinner-border-sm mx-1"
            role="status"
          ></span>
          {{ t("common.loadMore") }}
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useI18n } from "vue-i18n";
import EmptyState from "../../components/EmptyState.vue";
import { usePagedList } from "../../composables/usePagedList";
import { useToast } from "../../composables/useToast";
import type { SaleSession } from "../../types";
import { formatMoney } from "../../lib/currency";
import emptySessions from "../../assets/empty/sessions.svg";

const toast = useToast();
const { t, locale } = useI18n();

const search = ref("");
const filter = ref("");

const {
  items: sessions,
  loading,
  loadingMore,
  hasMore,
  reload,
  loadMore,
} = usePagedList<SaleSession>(
  (limit, offset) =>
    invoke<SaleSession[]>("list_sessions", {
      input: {
        status: filter.value || null,
        search: search.value.trim() || null,
        limit,
        offset,
      },
    }),
  [filter, search],
  (e) => toast.error(String(e)),
);

function fmt(n: number): string {
  return formatMoney(n);
}

function dateLabel(d: string): string {
  const date = new Date(d + (d.includes("T") ? "" : "Z"));
  if (isNaN(date.getTime())) return d;
  return date.toLocaleString(locale.value);
}

onMounted(reload);
</script>
