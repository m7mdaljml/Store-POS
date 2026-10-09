<template>
  <div class="container-fluid p-3">
    <div class="d-flex align-items-center justify-content-between mb-3">
      <h1 class="h4 mb-0">{{ t("nav.withdrawals") }}</h1>
      <div class="d-flex gap-2">
        <AsyncButton
          variant="outline-primary"
          :loading="exporting"
          @click="exportData"
        >
          <i v-if="!exporting" class="bi bi-file-earmark-excel mx-1"></i
          >{{ t("common.export") }}
        </AsyncButton>
        <button class="btn btn-primary" type="button" @click="openAdd">
          <i class="bi bi-plus-lg mx-1"></i>{{ t("withdrawals.newWithdrawal") }}
        </button>
      </div>
    </div>
    <div class="card mb-3">
      <div class="p-2 d-flex flex-wrap gap-2 align-items-center">
        <input
          v-model="filters.from"
          class="form-control form-control-sm"
          type="date"
          style="width: 160px"
        />
        <input
          v-model="filters.to"
          class="form-control form-control-sm"
          type="date"
          style="width: 160px"
        />
        <AppSelect
          v-model="filters.userId"
          sm
          class="app-select-inline"
          :items="userItems"
          :option-label="(s: any) => s.name"
          :option-value="(s: any) => s.id"
          :placeholder="t('withdrawals.enteredBy')"
        />
        <input
          v-model="filters.search"
          class="form-control form-control-sm"
          type="text"
          :placeholder="t('common.search')"
          style="width: 220px"
        />
        <button
          v-if="isFiltered"
          class="btn btn-sm btn-outline-secondary"
          type="button"
          @click="resetFilters"
        >
          <i class="bi bi-x-lg mx-1"></i>{{ t("common.clear") }}
        </button>
        <span class="ms-auto text-muted small">{{
          t("common.results", { count: filteredCount })
        }}</span>
      </div>
    </div>
    <div class="row g-2 mb-3">
      <div class="col">
        <div class="card text-center p-3 h-100">
          <div class="text-muted small text-uppercase">
            {{ t("withdrawals.totalWithdrawals") }}
          </div>
          <div class="fs-4 fw-semibold">{{ fmt(totalWithdrawals) }}</div>
        </div>
      </div>
    </div>
    <div class="card">
      <div class="table-responsive">
        <table class="table align-middle mb-0">
          <thead v-if="withdrawals.length">
            <tr>
              <th>{{ t("common.date") }}</th>
              <th>{{ t("withdrawals.enteredBy") }}</th>
              <th class="text-start">{{ t("withdrawals.amount") }}</th>
              <th>{{ t("withdrawals.notes") }}</th>
              <th class="text-start">{{ t("common.actions") }}</th>
            </tr>
          </thead>
          <tbody>
            <tr v-if="loading">
              <td colspan="5" class="text-center text-muted py-4">
                {{ t("common.loading") }}
              </td>
            </tr>
            <tr v-else-if="withdrawals.length === 0">
              <td colspan="5" class="text-center text-muted py-4">
                {{ t("withdrawals.noWithdrawals") }}
              </td>
            </tr>
            <tr v-for="w in withdrawals" :key="w.id">
              <td>{{ w.date || (w.createdAt ? w.createdAt.slice(0, 10) : "") }}</td>
              <td>{{ w.userName || w.userId || "—" }}</td>
              <td class="text-start fw-semibold">{{ fmt(w.amount) }}</td>
              <td>{{ w.notes }}</td>
              <td class="text-start">
                <button
                  class="btn btn-sm btn-outline-danger"
                  type="button"
                  :title="t('common.delete')"
                  @click="remove(w)"
                >
                  <i class="bi bi-trash"></i>
                </button>
              </td>
            </tr>
          </tbody>
          <tfoot v-if="withdrawals.length && !loading">
            <tr>
              <th colspan="2" class="text-start">
                {{ t("withdrawals.totalWithdrawals") }}
              </th>
              <th class="text-start fw-semibold">
                {{ fmt(totalWithdrawals) }}
              </th>
              <th></th>
              <th></th>
            </tr>
          </tfoot>
        </table>
      </div>
    </div>
    <div
      class="modal fade"
      :class="{ show: showAddModal, 'd-block': showAddModal }"
      tabindex="-1"
      @click.self="showAddModal = false"
    >
      <div class="modal-dialog modal-dialog-centered">
        <div class="modal-content">
          <div class="modal-header">
            <h5 class="modal-title">{{ t("withdrawals.newWithdrawal") }}</h5>
            <button
              type="button"
              class="btn-close"
              @click="showAddModal = false"
            ></button>
          </div>
          <div class="modal-body">
            <div class="mb-3">
              <label class="form-label" for="w-amount">{{
                t("withdrawals.amount")
              }}</label>
              <input
                id="w-amount"
                v-model="form.amount"
                class="form-control"
                :class="{ 'is-invalid': errors.amount }"
                type="number"
                step="0.01"
                min="0.01"
              />
              <div class="invalid-feedback">{{ errors.amount }}</div>
            </div>
            <div class="mb-3">
              <label class="form-label" for="w-date">{{
                t("common.date")
              }}</label>
              <input
                id="w-date"
                v-model="form.date"
                class="form-control"
                type="date"
              />
            </div>
            <div class="mb-3">
              <label class="form-label" for="w-notes">{{
                t("withdrawals.notes")
              }}</label>
              <textarea
                id="w-notes"
                v-model="form.notes"
                class="form-control"
                rows="2"
              ></textarea>
            </div>
          </div>
          <div class="modal-footer">
            <button
              type="button"
              class="btn btn-secondary"
              @click="showAddModal = false"
            >
              {{ t("common.cancel") }}
            </button>
            <AsyncButton :loading="saving" @click="submitAdd">{{
              t("withdrawals.addWithdrawal")
            }}</AsyncButton>
          </div>
        </div>
      </div>
    </div>
    <div v-if="showAddModal" class="modal-backdrop fade show"></div>
  </div>
</template>
<script setup lang="ts">
import { ref, computed, onMounted, watch } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { save as saveDialog } from "@tauri-apps/plugin-dialog";
import { useAuthStore } from "../../stores/auth";
import AppSelect from "../../components/AppSelect.vue";
import AsyncButton from "../../components/AsyncButton.vue";
import { formatMoney } from "../../lib/currency";
import { useToast } from "../../composables/useToast";
import { useConfirm } from "../../composables/useConfirm";
import type { Paged } from "../../composables/usePagedList";
import type { UserRecord } from "../../types";

const { t } = useI18n();
const authStore = useAuthStore();
const toast = useToast();
const { confirmDialog } = useConfirm();
const fmt = (v: number | null | undefined) => formatMoney(v ?? 0);

interface Withdrawal {
  id: number;
  sessionId: number | null;
  userId: number | null;
  userName: string | null;
  amount: number;
  date: string | null;
  notes: string | null;
  createdAt: string;
}
interface User {
  id: number;
  name: string;
}
const loading = ref(false);
const exporting = ref(false);
const withdrawals = ref<Withdrawal[]>([]);
const totalCount = ref(0);
const limit = ref(20);
const offset = ref(0);
const users = ref<User[]>([]);
const showAddModal = ref(false);
const saving = ref(false);
const form = ref({
  amount: "",
  userId: authStore.user?.id ?? null,
  date: new Date().toISOString().slice(0, 10),
  notes: "",
});
const errors = ref<Record<string, string>>({});

const today = () => new Date().toISOString().slice(0, 10);
function defaultFilters() {
  return { from: today(), to: today(), userId: null as number | null, search: "" };
}
const filters = ref(defaultFilters());

const userItems = computed<any[]>(() =>
  users.value.map((u) => ({ id: u.id, name: u.name })),
);
const isFiltered = computed(() => {
  const d = defaultFilters();
  return (
    filters.value.from !== d.from ||
    filters.value.to !== d.to ||
    filters.value.userId != null ||
    !!filters.value.search
  );
});
function resetFilters() {
  filters.value = defaultFilters();
}
const totalWithdrawals = computed(() =>
  withdrawals.value.reduce((s, w) => s + (w.amount ?? 0), 0),
);
const filteredCount = computed(() => totalCount.value);

async function loadUsers() {
  try {
    const res = await invoke<Paged<UserRecord>>("list_users", {
      limit: 1000,
      offset: 0,
    });
    users.value = (res.items ?? [])
      .filter((u) => u.isActive)
      .map((u) => ({ id: u.id, name: u.fullName }));
  } catch {}
}
async function loadData() {
  loading.value = true;
  try {
    const res = await invoke<Paged<Withdrawal>>("list_cash_withdrawals", {
      limit: limit.value,
      offset: offset.value,
      sessionId: null,
      userId: filters.value.userId,
      from: filters.value.from || null,
      to: filters.value.to || null,
      search: filters.value.search || null,
    });
    withdrawals.value = res.items ?? [];
    totalCount.value = res.total ?? 0;
  } catch (e) {
    toast.error(e instanceof Error ? e.message : String(e));
  } finally {
    loading.value = false;
  }
}
function openAdd() {
  form.value = {
    amount: "",
    userId: authStore.user?.id ?? null,
    date: new Date().toISOString().slice(0, 10),
    notes: "",
  };
  errors.value = {};
  showAddModal.value = true;
}
async function submitAdd() {
  saving.value = true;
  errors.value = {};
  try {
    const amountNum = parseFloat(form.value.amount);
    if (isNaN(amountNum) || amountNum <= 0) {
      errors.value.amount = t("withdrawals.invalidAmount");
      return;
    }
    if (!form.value.userId) return;
    if (!(await confirmDialog({ message: t("withdrawals.confirmAdd") }))) {
      return;
    }
    await invoke("add_cash_withdrawal", {
      input: {
        amount: amountNum,
        userId: form.value.userId,
        date: form.value.date || null,
        notes: form.value.notes || null,
      },
    });
    toast.success(t("withdrawals.added"));
    showAddModal.value = false;
    await loadData();
  } catch (e) {
    toast.error(e instanceof Error ? e.message : String(e));
  } finally {
    saving.value = false;
  }
}
async function remove(w: Withdrawal) {
  const ok = await confirmDialog({
    message: t("withdrawals.deleteConfirm", { amount: fmt(w.amount) }),
  });
  if (!ok) return;
  try {
    await invoke("delete_cash_withdrawal", {
      withdrawalId: w.id,
      userId: authStore.user?.id ?? null,
    });
    toast.success(t("withdrawals.deleted"));
    await loadData();
  } catch (e) {
    toast.error(e instanceof Error ? e.message : String(e));
  }
}
async function exportData() {
  exporting.value = true;
  try {
    const path = await saveDialog({
      title: t("withdrawals.exportTitle"),
      defaultPath: `withdrawals-${new Date().toISOString().slice(0, 10)}.xlsx`,
      filters: [{ name: t("withdrawals.excelFilter"), extensions: ["xlsx"] }],
    });
    if (!path) return;
    await invoke("export_cash_withdrawals", {
      path,
      from: filters.value.from || null,
      to: filters.value.to || null,
      userId: filters.value.userId,
    });
    toast.success(t("withdrawals.exported", { path }));
  } catch (e) {
    toast.error(e instanceof Error ? e.message : String(e));
  } finally {
    exporting.value = false;
  }
}
watch(
  () => [
    filters.value.from,
    filters.value.to,
    filters.value.userId,
    filters.value.search,
  ],
  () => {
    offset.value = 0;
    loadData();
  },
);
onMounted(async () => {
  await loadUsers();
  await loadData();
});
</script>
