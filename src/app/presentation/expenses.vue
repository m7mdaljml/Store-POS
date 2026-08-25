<template>
  <div>
    <div class="d-flex align-items-center justify-content-between mb-3">
      <h1 class="h4 mb-0">{{ t("expenses.title") }}</h1>
      <div class="d-flex gap-2">
        <AsyncButton
          v-can="'export.excel'"
          variant="outline-primary"
          :loading="exporting"
          @click="exportExcel"
        >
          <i v-if="!exporting" class="bi bi-file-earmark-excel mx-1"></i
          >{{ t("common.export") }}
        </AsyncButton>
        <button
          class="btn btn-outline-secondary"
          type="button"
          @click="showCatsModal = true"
        >
          <i class="bi bi-tags mx-1"></i>{{ t("expenses.categories") }}
        </button>
        <button class="btn btn-primary" type="button" @click="openAdd">
          <i class="bi bi-plus-lg mx-1"></i>{{ t("expenses.newExpense") }}
        </button>
      </div>
    </div>

    <div class="row g-2 mb-3">
      <div class="col">
        <div class="card text-center p-3 h-100">
          <div class="text-muted small text-uppercase">
            {{ t("expenses.outstandingSupplierDues") }}
          </div>
          <div
            class="fs-4 fw-semibold"
            :class="(summary?.outstanding_due ?? 0) > 0 ? 'text-danger' : ''"
          >
            {{ fmt(summary?.outstanding_due ?? 0) }}
          </div>
        </div>
      </div>
      <div class="col">
        <div class="card text-center p-3 h-100">
          <div class="text-muted small text-uppercase">
            {{ t("expenses.incomingPeriod") }}
          </div>
          <div class="fs-4 fw-semibold">{{ fmt(summary?.total_in ?? 0) }}</div>
          <div class="text-muted small">
            {{
              t(
                "expenses.invoiceCount",
                { count: summary?.incoming_count ?? 0 },
                summary?.incoming_count ?? 0,
              )
            }}
          </div>
        </div>
      </div>
      <div class="col">
        <div class="card text-center p-3 h-100">
          <div class="text-muted small text-uppercase">
            {{ t("expenses.outgoingPeriod") }}
          </div>
          <div class="fs-4 fw-semibold text-danger">
            {{ fmt(summary?.total_out ?? 0) }}
          </div>
          <div class="text-muted small">
            {{
              t(
                "expenses.expenseCount",
                { count: summary?.outgoing_count ?? 0 },
                summary?.outgoing_count ?? 0,
              )
            }}
          </div>
        </div>
      </div>
      <div class="col">
        <div class="card text-center p-3 h-100">
          <div class="text-muted small text-uppercase">
            {{ t("expenses.netPeriod") }}
          </div>
          <div
            class="fs-4 fw-semibold"
            :class="
              (summary?.total_in ?? 0) - (summary?.total_out ?? 0) < 0
                ? 'text-danger'
                : ''
            "
          >
            {{ fmt((summary?.total_in ?? 0) - (summary?.total_out ?? 0)) }}
          </div>
        </div>
      </div>
    </div>

    <div class="card mb-3">
      <div class="p-2 d-flex flex-wrap gap-2 align-items-center">
        <select
          v-model="filters.kind"
          class="form-select form-select-sm"
          style="width: auto"
          :aria-label="t('expenses.filterTypeAria')"
        >
          <option value="all">{{ t("expenses.allTypes") }}</option>
          <option value="in">{{ t("expenses.incomingStockIn") }}</option>
          <option value="out">{{ t("expenses.outgoingMoneyOut") }}</option>
        </select>
        <AppSelect
          v-model="filters.supplierId"
          sm
          class="app-select-inline"
          :items="suppliers"
          :option-label="(s) => s.name"
          :option-value="(s) => s.id"
          :placeholder="t('expenses.allSuppliers')"
          :aria-label="t('expenses.filterSupplierAria')"
        />
        <select
          v-model="filters.status"
          class="form-select form-select-sm"
          style="width: auto"
          :aria-label="t('expenses.filterStatusAria')"
        >
          <option value="all">{{ t("expenses.allStatuses") }}</option>
          <option value="unpaid">{{ t("expenses.unpaid") }}</option>
          <option value="partial">{{ t("expenses.partial") }}</option>
          <option value="paid">{{ t("common.paid") }}</option>
        </select>
        <input
          v-model="filters.from"
          class="form-control form-control-sm"
          type="date"
          style="width: auto"
          :aria-label="t('expenses.fromDateAria')"
        />
        <span class="text-muted small">→</span>
        <input
          v-model="filters.to"
          class="form-control form-control-sm"
          type="date"
          style="width: auto"
          :aria-label="t('expenses.toDateAria')"
        />
        <input
          v-model="search"
          class="form-control form-control-sm flex-grow-1"
          type="search"
          :placeholder="t('expenses.searchPlaceholder')"
        />
        <button
          v-if="
            filters.kind !== 'all' ||
            filters.supplierId != null ||
            filters.status !== 'all' ||
            filters.from ||
            filters.to
          "
          class="btn btn-sm btn-outline-secondary"
          type="button"
          @click="
            filters = {
              kind: 'all',
              supplierId: null,
              status: 'all',
              from: '',
              to: '',
            }
          "
        >
          <i class="bi bi-x-lg mx-1"></i>{{ t("common.clear") }}
        </button>
        <span class="ms-auto text-muted small">
          {{ t("common.results", { count: filteredCount }, filteredCount) }}
        </span>
      </div>
    </div>

    <div class="card">
      <div class="table-responsive">
        <table class="table align-middle mb-0">
          <thead v-if="expenses.length">
            <tr>
              <th>{{ t("expenses.type") }}</th>
              <th>{{ t("expenses.refInvoice") }}</th>
              <th>{{ t("suppliers.supplier") }}</th>
              <th>{{ t("common.date") }}</th>
              <th>{{ t("expenses.details") }}</th>
              <th class="text-end">{{ t("common.amount") }}</th>
              <th class="text-end">{{ t("common.paid") }}</th>
              <th class="text-end">{{ t("common.due") }}</th>
              <th>{{ t("common.status") }}</th>
            </tr>
          </thead>
          <tbody v-if="loading">
            <tr>
              <td colspan="9" class="text-center text-muted py-4">
                {{ t("common.loading") }}
              </td>
            </tr>
          </tbody>
          <tbody v-else-if="!expenses.length">
            <tr>
              <td colspan="9" class="p-0 border-0">
                <EmptyState
                  :image="emptyExpenses"
                  :message="t('expenses.noExpenses')"
                />
              </td>
            </tr>
          </tbody>
          <tbody v-for="e in expenses" :key="e.kind + '-' + e.id">
            <tr>
              <td>
                <span
                  class="badge"
                  :class="
                    e.kind === 'in' ? 'text-bg-primary' : 'text-bg-secondary'
                  "
                >
                  {{ e.kind === "in" ? t("expenses.in") : t("expenses.out") }}
                </span>
              </td>
              <td class="fw-semibold">{{ e.ref_no ?? "—" }}</td>
              <td>{{ e.supplier_name ?? "—" }}</td>
              <td class="text-muted">{{ e.date }}</td>
              <td class="text-muted">{{ e.notes ?? "—" }}</td>
              <td
                class="text-end fw-semibold"
                :class="e.kind === 'out' ? 'text-danger' : ''"
              >
                {{ fmt(e.amount) }}
              </td>
              <td class="text-end">
                {{ e.kind === "in" ? fmt(e.paid_amount) : "—" }}
              </td>
              <td class="text-end">
                <span
                  v-if="e.kind === 'in' && e.due_amount > 0"
                  class="text-danger fw-semibold"
                >
                  {{ fmt(e.due_amount) }}
                </span>
                <span v-else>—</span>
              </td>
              <td>
                <span class="badge" :class="statusBadge(e.status)">
                  {{ statusLabel(e.status) }}
                </span>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
      <Paginator
        :page="page"
        :page-size="size"
        :total-items="totalItems"
        :disabled="loading"
        @update:page="goToPage"
      />
    </div>

    <div v-if="showModal" class="modal-backdrop show"></div>
    <div v-if="showModal" class="modal d-block" tabindex="-1">
      <div class="modal-dialog modal-dialog-centered">
        <div class="modal-content">
          <form @submit.prevent="save">
            <div class="modal-header">
              <h5 class="modal-title">
                {{ t("expenses.newOutgoingExpense") }}
              </h5>
              <button
                type="button"
                class="btn-close"
                @click="showModal = false"
              ></button>
            </div>
            <div class="modal-body">
              <div
                v-if="formError"
                class="alert alert-danger py-2 small"
                role="alert"
              >
                <i class="bi bi-exclamation-triangle mx-1"></i>{{ formError }}
              </div>
              <div class="mb-3">
                <label class="form-label" for="e-amount">{{
                  t("expenses.amount")
                }}</label>
                <input
                  id="e-amount"
                  v-model.number="form.amount"
                  class="form-control"
                  :class="{ 'is-invalid': errors.amount }"
                  type="number"
                  step="0.01"
                  min="0.01"
                  autofocus
                  @input="clearFieldError(errors, 'amount')"
                />
                <div class="invalid-feedback">{{ errors.amount }}</div>
              </div>
              <div class="row g-3">
                <div class="col-md-6">
                  <label class="form-label" for="e-cat">{{
                    t("expenses.category")
                  }}</label>
                  <AppSelect
                    id="e-cat"
                    v-model="form.categoryId"
                    :items="categoryOptions"
                    :option-label="(c) => c.name"
                    :option-value="(c) => c.id"
                  />
                </div>
                <div class="col-md-6">
                  <label class="form-label" for="e-date">{{
                    t("common.date")
                  }}</label>
                  <input
                    id="e-date"
                    v-model="form.date"
                    class="form-control"
                    :class="{ 'is-invalid': errors.date }"
                    type="date"
                    @input="clearFieldError(errors, 'date')"
                  />
                  <div class="invalid-feedback">{{ errors.date }}</div>
                </div>
              </div>
              <div class="mt-3 mb-3">
                <label class="form-label" for="e-desc">{{
                  t("expenses.description")
                }}</label>
                <textarea
                  id="e-desc"
                  v-model="form.description"
                  class="form-control"
                  rows="2"
                  :placeholder="t('expenses.descriptionPlaceholder')"
                ></textarea>
              </div>
              <div class="mb-0">
                <label class="form-label" for="e-ref">{{
                  t("expenses.referenceNo")
                }}</label>
                <input
                  id="e-ref"
                  v-model="form.referenceNo"
                  class="form-control"
                  type="text"
                  :placeholder="t('expenses.refPlaceholder')"
                />
              </div>
              <div
                class="d-flex justify-content-end gap-2 mt-3 pt-3 border-top"
              >
                <button
                  type="button"
                  class="btn btn-outline-secondary"
                  @click="showModal = false"
                >
                  {{ t("common.cancel") }}
                </button>
                <AsyncButton
                  type="submit"
                  :loading="saving"
                  :disabled="!canSave"
                >
                  {{ t("expenses.saveExpense") }}
                </AsyncButton>
              </div>
            </div>
          </form>
        </div>
      </div>
    </div>

    <div v-if="showCatsModal" class="modal-backdrop show"></div>
    <div v-if="showCatsModal" class="modal d-block" tabindex="-1">
      <div class="modal-dialog modal-dialog-centered">
        <div class="modal-content">
          <div class="modal-header">
            <h5 class="modal-title">{{ t("expenses.expenseCategories") }}</h5>
            <button
              type="button"
              class="btn-close"
              @click="showCatsModal = false"
            ></button>
          </div>
          <div class="modal-body">
            <div
              v-if="categoryError"
              class="alert alert-danger py-2 small"
              role="alert"
            >
              <i class="bi bi-exclamation-triangle mx-1"></i>{{ categoryError }}
            </div>
            <form class="d-flex gap-2 mb-3" @submit.prevent="addCategory">
              <input
                v-model="newCategoryName"
                class="form-control"
                :class="{ 'is-invalid': catErrors.name }"
                type="text"
                :placeholder="t('expenses.newCategoryName')"
                :aria-label="t('expenses.newCategoryName')"
                @input="clearFieldError(catErrors, 'name')"
              />
              <div class="invalid-feedback d-block" v-if="catErrors.name">
                {{ catErrors.name }}
              </div>
              <AsyncButton
                type="submit"
                variant="outline-primary"
                :loading="catSaving"
              >
                {{ t("common.add") }}
              </AsyncButton>
            </form>
            <div v-if="!categories.length" class="text-muted small">
              {{ t("expenses.noCategoriesYet") }}
            </div>
            <ul class="list-group">
              <li
                v-for="c in categories"
                :key="c.id"
                class="list-group-item d-flex justify-content-between align-items-center"
              >
                <div>
                  <span class="fw-semibold">{{ c.name }}</span>
                  <span class="text-muted small ms-2">
                    {{
                      t(
                        "expenses.expenseCount",
                        { count: c.expenseCount },
                        c.expenseCount,
                      )
                    }}
                  </span>
                </div>
                <button
                  class="btn btn-sm btn-outline-danger"
                  type="button"
                  :title="t('common.delete')"
                  @click="removeCategory(c)"
                >
                  <i class="bi bi-trash"></i>
                </button>
              </li>
            </ul>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { save as saveDialog } from "@tauri-apps/plugin-dialog";
import { useI18n } from "vue-i18n";
import EmptyState from "../../components/EmptyState.vue";
import AsyncButton from "../../components/AsyncButton.vue";
import Paginator from "../../components/Paginator.vue";
import AppSelect from "../../components/AppSelect.vue";
import {
  applyFieldRules,
  clearFieldError,
  useFormGuard,
} from "../../composables/useFormGuard";
import { usePagedList, type Paged } from "../../composables/usePagedList";
import { useToast } from "../../composables/useToast";
import { useConfirm } from "../../composables/useConfirm";
import { useSettingsStore } from "../../stores/settings";
import { useAuth } from "../../composables/useAuth";
import type {
  ExpenseCategory,
  ExpenseRecord,
  ExpenseSummary,
  Supplier,
} from "../../types";
import emptyExpenses from "../../assets/empty/expenses.svg";
import { formatMoney } from "../../lib/currency";

const settings = useSettingsStore();
const auth = useAuth();
const toast = useToast();
const { confirmDialog } = useConfirm();
const { t } = useI18n();

const suppliers = ref<Supplier[]>([]);
const categories = ref<ExpenseCategory[]>([]);
const summary = ref<ExpenseSummary | null>(null);
const exporting = ref(false);
const search = ref("");

const filters = ref({
  kind: "all" as "all" | "in" | "out",
  supplierId: null as number | null,
  status: "all",
  from: "",
  to: "",
});

const {
  items: expenses,
  loading,
  page,
  size,
  totalItems,
  goToPage,
  reload: reloadExpenses,
} = usePagedList<ExpenseRecord>(
  (limit, offset) =>
    invoke<Paged<ExpenseRecord>>("list_expenses", {
      ...filterBase(),
      search: search.value.trim() || null,
      limit,
      offset,
    }),
  [filters, search],
  (e) => toast.error(e instanceof Error ? e.message : String(e)),
);

const showModal = ref(false);
const saving = ref(false);
const formError = ref("");
const form = ref({
  categoryId: null as number | null,
  amount: 0 as number | null,
  date: new Date().toISOString().slice(0, 10),
  description: "",
  referenceNo: "",
});
const errors = reactive<Record<string, string>>({});
const guard = useFormGuard(form);
const canSave = computed(() => guard.isDirty.value && !saving.value);

function resetErrors() {
  for (const key of Object.keys(errors)) delete errors[key];
}

const showCatsModal = ref(false);
const catSaving = ref(false);
const categoryError = ref("");
const newCategoryName = ref("");
const catErrors = reactive<Record<string, string>>({});

const filteredCount = computed(() => expenses.value.length);

function fmt(n: number): string {
  return formatMoney(n);
}

function statusLabel(status: string): string {
  switch (status) {
    case "paid":
      return t("common.paid");
    case "partial":
      return t("expenses.partial");
    case "unpaid":
      return t("expenses.unpaid");
    default:
      return status;
  }
}

function filterBase() {
  return {
    kind: filters.value.kind === "all" ? null : filters.value.kind,
    supplierId: filters.value.supplierId,
    status: filters.value.status === "all" ? null : filters.value.status,
    from: filters.value.from || null,
    to: filters.value.to || null,
  };
}

/** Suppliers/categories dropdowns + summary cards; the record list itself is
 *  paginated through `usePagedList`. */
async function loadMeta() {
  try {
    const [s, c, sum] = await Promise.all([
      invoke<Paged<Supplier>>("list_suppliers"),
      invoke<ExpenseCategory[]>("list_expense_categories"),
      invoke<ExpenseSummary>("expense_summary", {
        from: filters.value.from || null,
        to: filters.value.to || null,
      }),
    ]);
    suppliers.value = s.items;
    categories.value = c;
    summary.value = sum;
  } catch (e) {
    toast.error(e instanceof Error ? e.message : String(e));
  }
}

/** Category dropdown options: a leading "None" entry plus the DB list. */
const categoryOptions = computed<{ id: number | null; name: string }[]>(() => [
  { id: null, name: t("common.none") },
  ...categories.value.map((c) => ({ id: c.id, name: c.name })),
]);

function openAdd() {
  formError.value = "";
  form.value = {
    categoryId: categories.value[0]?.id ?? null,
    amount: null,
    date: new Date().toISOString().slice(0, 10),
    description: "",
    referenceNo: "",
  };
  resetErrors();
  guard.capture();
  showModal.value = true;
}

function validate(): boolean {
  const amount = form.value.amount;
  const ok = applyFieldRules(errors, [
    [
      "amount",
      typeof amount === "number" && !isNaN(amount) && amount > 0,
      t("expenses.amount"),
    ],
    ["date", !!form.value.date, t("common.date")],
  ]);
  return ok;
}

function payload() {
  return {
    categoryId: form.value.categoryId,
    amount: form.value.amount,
    date: form.value.date,
    description: form.value.description || null,
    referenceNo: form.value.referenceNo || null,
    userId: auth.user?.id ?? null,
  };
}

async function save() {
  formError.value = "";
  if (!validate()) {
    toast.error(t("common.fixErrors"));
    return;
  }
  saving.value = true;
  try {
    await invoke<number>("add_expense_out", { input: payload() });
    guard.markSaved();
    showModal.value = false;
    toast.success(t("expenses.expenseRecorded"));
    await Promise.all([reloadExpenses(), loadMeta()]);
  } catch (e) {
    formError.value = e instanceof Error ? e.message : String(e);
  } finally {
    saving.value = false;
  }
}

async function addCategory() {
  categoryError.value = "";
  if (
    !applyFieldRules(catErrors, [
      ["name", !!newCategoryName.value.trim(), t("expenses.category")],
    ])
  ) {
    toast.error(t("common.fixErrors"));
    return;
  }
  catSaving.value = true;
  try {
    await invoke<number>("create_expense_category", {
      name: newCategoryName.value,
    });
    newCategoryName.value = "";
    toast.success(t("expenses.categoryAdded"));
    await loadMeta();
  } catch (e) {
    categoryError.value = e instanceof Error ? e.message : String(e);
  } finally {
    catSaving.value = false;
  }
}

async function removeCategory(c: ExpenseCategory) {
  categoryError.value = "";
  if (
    !(await confirmDialog({
      message: t("expenses.deleteCategoryConfirm", { name: c.name }),
    }))
  )
    return;
  try {
    await invoke("delete_expense_category", { categoryId: c.id });
    toast.success(t("expenses.categoryDeleted", { name: c.name }));
    if (form.value.categoryId === c.id) form.value.categoryId = null;
    await loadMeta();
  } catch (e) {
    categoryError.value = e instanceof Error ? e.message : String(e);
  }
}

function statusBadge(status: string) {
  switch (status) {
    case "paid":
      return "text-bg-success";
    case "partial":
      return "text-bg-warning";
    case "unpaid":
      return "text-bg-danger";
    default:
      return "text-bg-secondary";
  }
}

async function exportExcel() {
  exporting.value = true;
  try {
    const path = await saveDialog({
      title: t("expenses.exportTitle"),
      defaultPath: `expenses-${new Date().toISOString().slice(0, 10)}.xlsx`,
      filters: [{ name: t("expenses.excelFilter"), extensions: ["xlsx"] }],
    });
    if (!path) return;
    await invoke("export_expenses", { path, ...filterBase() });
    toast.success(t("expenses.exportedTo", { path }));
  } catch (e) {
    toast.error(e instanceof Error ? e.message : String(e));
  } finally {
    exporting.value = false;
  }
}

watch(filters, () => loadMeta(), { deep: true });

onMounted(async () => {
  await Promise.allSettled([reloadExpenses(), loadMeta(), settings.load()]);
});
</script>

<style scoped>
.app-select-inline {
  width: auto;
  min-width: 200px;
  max-width: 260px;
}
</style>
