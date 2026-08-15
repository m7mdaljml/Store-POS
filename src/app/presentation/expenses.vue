<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useSettingsStore } from "../../stores/settings";
import { useAuth } from "../../composables/useAuth";
import type {
  ExpenseCategory,
  ExpenseRecord,
  ExpenseSummary,
  Supplier,
} from "../../types";

const settings = useSettingsStore();
const auth = useAuth();

const suppliers = ref<Supplier[]>([]);
const categories = ref<ExpenseCategory[]>([]);
const expenses = ref<ExpenseRecord[]>([]);
const summary = ref<ExpenseSummary | null>(null);
const loading = ref(false);
const error = ref("");
const notice = ref("");

const filters = ref({
  kind: "all" as "all" | "in" | "out",
  supplierId: null as number | null,
  status: "all",
  from: "",
  to: "",
});

const showModal = ref(false);
const saving = ref(false);
const formError = ref("");
const form = ref({
  categoryId: null as number | null,
  amount: 0,
  date: new Date().toISOString().slice(0, 10),
  description: "",
  referenceNo: "",
});

const showCatsModal = ref(false);
const catSaving = ref(false);
const categoryError = ref("");
const newCategoryName = ref("");

const filteredCount = computed(() => expenses.value.length);

function fmt(n: number): string {
  if (!settings.currency) return n.toFixed(2);
  return new Intl.NumberFormat(undefined, {
    style: "currency",
    currency: settings.currency,
    currencyDisplay: "narrowSymbol",
  }).format(n);
}

function filterParams() {
  return {
    kind: filters.value.kind === "all" ? null : filters.value.kind,
    supplierId: filters.value.supplierId,
    status: filters.value.status === "all" ? null : filters.value.status,
    from: filters.value.from || null,
    to: filters.value.to || null,
  };
}

async function load() {
  loading.value = true;
  error.value = "";
  try {
    const [s, c, e, sum] = await Promise.all([
      invoke<Supplier[]>("list_suppliers"),
      invoke<ExpenseCategory[]>("list_expense_categories"),
      invoke<ExpenseRecord[]>("list_expenses", filterParams()),
      invoke<ExpenseSummary>("expense_summary", {
        from: filters.value.from || null,
        to: filters.value.to || null,
      }),
    ]);
    suppliers.value = s;
    categories.value = c;
    expenses.value = e;
    summary.value = sum;
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

function openAdd() {
  formError.value = "";
  form.value = {
    categoryId: categories.value[0]?.id ?? null,
    amount: 0,
    date: new Date().toISOString().slice(0, 10),
    description: "",
    referenceNo: "",
  };
  showModal.value = true;
}

function validate(): string {
  const amount = form.value.amount;
  if (typeof amount !== "number" || isNaN(amount) || amount <= 0)
    return "Enter an amount greater than zero";
  return "";
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
  const err = validate();
  if (err) {
    formError.value = err;
    return;
  }
  saving.value = true;
  try {
    await invoke<number>("add_expense_out", { input: payload() });
    showModal.value = false;
    notice.value = "Expense recorded";
    await load();
  } catch (e) {
    formError.value = e instanceof Error ? e.message : String(e);
  } finally {
    saving.value = false;
  }
}

async function addCategory() {
  categoryError.value = "";
  if (!newCategoryName.value.trim()) {
    categoryError.value = "Category name is required";
    return;
  }
  catSaving.value = true;
  try {
    await invoke<number>("create_expense_category", { name: newCategoryName.value });
    newCategoryName.value = "";
    notice.value = "Category added";
    await load();
  } catch (e) {
    categoryError.value = e instanceof Error ? e.message : String(e);
  } finally {
    catSaving.value = false;
  }
}

async function removeCategory(c: ExpenseCategory) {
  categoryError.value = "";
  if (!window.confirm(`Delete category "${c.name}"?`)) return;
  try {
    await invoke("delete_expense_category", { categoryId: c.id });
    notice.value = `"${c.name}" deleted`;
    if (form.value.categoryId === c.id) form.value.categoryId = null;
    await load();
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

watch(filters, () => load(), { deep: true });

onMounted(async () => {
  await Promise.allSettled([load(), settings.load()]);
});
</script>

<template>
  <div>
    <div class="d-flex align-items-center justify-content-between mb-3">
      <h1 class="h4 mb-0">Expenses</h1>
      <div class="d-flex gap-2">
        <button class="btn btn-outline-secondary" type="button" @click="showCatsModal = true">
          <i class="bi bi-tags me-1"></i>Categories
        </button>
        <button class="btn btn-primary" type="button" @click="openAdd">
          <i class="bi bi-plus-lg me-1"></i>New Expense
        </button>
      </div>
    </div>

    <div v-if="error" class="alert alert-danger py-2 small" role="alert">
      <i class="bi bi-exclamation-triangle me-1"></i>{{ error }}
    </div>
    <div v-if="notice" class="alert alert-success py-2 small" role="alert">
      <i class="bi bi-check-circle me-1"></i>{{ notice }}
    </div>

    <div class="row g-2 mb-3">
      <div class="col">
        <div class="card text-center p-3 h-100">
          <div class="text-muted small text-uppercase">Outstanding supplier dues</div>
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
          <div class="text-muted small text-uppercase">Incoming (period)</div>
          <div class="fs-4 fw-semibold">{{ fmt(summary?.total_in ?? 0) }}</div>
          <div class="text-muted small">{{ summary?.incoming_count ?? 0 }} invoice(s)</div>
        </div>
      </div>
      <div class="col">
        <div class="card text-center p-3 h-100">
          <div class="text-muted small text-uppercase">Outgoing (period)</div>
          <div class="fs-4 fw-semibold text-danger">{{ fmt(summary?.total_out ?? 0) }}</div>
          <div class="text-muted small">{{ summary?.outgoing_count ?? 0 }} expense(s)</div>
        </div>
      </div>
      <div class="col">
        <div class="card text-center p-3 h-100">
          <div class="text-muted small text-uppercase">Net (period)</div>
          <div
            class="fs-4 fw-semibold"
            :class="((summary?.total_in ?? 0) - (summary?.total_out ?? 0)) < 0 ? 'text-danger' : ''"
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
          aria-label="Filter by type"
        >
          <option value="all">All types</option>
          <option value="in">Incoming (stock-in)</option>
          <option value="out">Outgoing (money out)</option>
        </select>
        <select
          v-model="filters.supplierId"
          class="form-select form-select-sm"
          style="width: auto"
          aria-label="Filter by supplier"
        >
          <option :value="null">All suppliers</option>
          <option v-for="s in suppliers" :key="s.id" :value="s.id">{{ s.name }}</option>
        </select>
        <select
          v-model="filters.status"
          class="form-select form-select-sm"
          style="width: auto"
          aria-label="Filter by status"
        >
          <option value="all">All statuses</option>
          <option value="unpaid">Unpaid</option>
          <option value="partial">Partial</option>
          <option value="paid">Paid</option>
        </select>
        <input
          v-model="filters.from"
          class="form-control form-control-sm"
          type="date"
          style="width: auto"
          aria-label="From date"
        />
        <span class="text-muted small">→</span>
        <input
          v-model="filters.to"
          class="form-control form-control-sm"
          type="date"
          style="width: auto"
          aria-label="To date"
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
            filters = { kind: 'all', supplierId: null, status: 'all', from: '', to: '' }
          "
        >
          <i class="bi bi-x-lg me-1"></i>Clear
        </button>
        <span class="ms-auto text-muted small">{{ filteredCount }} result(s)</span>
      </div>
    </div>

    <div class="card">
      <div class="table-responsive">
        <table class="table align-middle mb-0">
          <thead>
            <tr>
              <th>Type</th>
              <th>Ref / Invoice</th>
              <th>Supplier</th>
              <th>Date</th>
              <th>Details</th>
              <th class="text-end">Amount</th>
              <th class="text-end">Paid</th>
              <th class="text-end">Due</th>
              <th>Status</th>
            </tr>
          </thead>
          <tbody>
            <tr v-if="loading">
              <td colspan="9" class="text-center text-muted py-4">Loading…</td>
            </tr>
            <tr v-else-if="!expenses.length">
              <td colspan="9" class="text-center text-muted py-4">
                No expenses match the current filters
              </td>
            </tr>
            <tr v-for="e in expenses" :key="e.kind + '-' + e.id">
              <td>
                <span class="badge" :class="e.kind === 'in' ? 'text-bg-primary' : 'text-bg-secondary'">
                  {{ e.kind === "in" ? "In" : "Out" }}
                </span>
              </td>
              <td class="fw-semibold">{{ e.ref_no ?? "—" }}</td>
              <td>{{ e.supplier_name ?? "—" }}</td>
              <td class="text-muted">{{ e.date }}</td>
              <td class="text-muted">{{ e.notes ?? "—" }}</td>
              <td class="text-end fw-semibold" :class="e.kind === 'out' ? 'text-danger' : ''">
                {{ fmt(e.amount) }}
              </td>
              <td class="text-end">{{ e.kind === "in" ? fmt(e.paid_amount) : "—" }}</td>
              <td class="text-end">
                <span v-if="e.kind === 'in' && e.due_amount > 0" class="text-danger fw-semibold">
                  {{ fmt(e.due_amount) }}
                </span>
                <span v-else>—</span>
              </td>
              <td>
                <span class="badge" :class="statusBadge(e.status)">
                  {{ e.status }}
                </span>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <div v-if="showModal" class="modal-backdrop show"></div>
    <div v-if="showModal" class="modal d-block" tabindex="-1">
      <div class="modal-dialog modal-dialog-centered">
        <div class="modal-content">
          <form @submit.prevent="save">
            <div class="modal-header">
              <h5 class="modal-title">New Outgoing Expense</h5>
              <button type="button" class="btn-close" @click="showModal = false"></button>
            </div>
            <div class="modal-body">
              <div v-if="formError" class="alert alert-danger py-2 small" role="alert">
                <i class="bi bi-exclamation-triangle me-1"></i>{{ formError }}
              </div>
              <div class="mb-3">
                <label class="form-label" for="e-amount">Amount *</label>
                <input
                  id="e-amount"
                  v-model.number="form.amount"
                  class="form-control"
                  type="number"
                  step="0.01"
                  min="0.01"
                  autofocus
                />
              </div>
              <div class="row g-3">
                <div class="col-md-6">
                  <label class="form-label" for="e-cat">Category</label>
                  <select id="e-cat" v-model="form.categoryId" class="form-select">
                    <option :value="null">None</option>
                    <option v-for="c in categories" :key="c.id" :value="c.id">{{ c.name }}</option>
                  </select>
                </div>
                <div class="col-md-6">
                  <label class="form-label" for="e-date">Date</label>
                  <input id="e-date" v-model="form.date" class="form-control" type="date" />
                </div>
              </div>
              <div class="mt-3 mb-3">
                <label class="form-label" for="e-desc">Description</label>
                <textarea
                  id="e-desc"
                  v-model="form.description"
                  class="form-control"
                  rows="2"
                  placeholder="e.g. Rent, utilities, wages, transport…"
                ></textarea>
              </div>
              <div class="mb-0">
                <label class="form-label" for="e-ref">Reference no.</label>
                <input
                  id="e-ref"
                  v-model="form.referenceNo"
                  class="form-control"
                  type="text"
                  placeholder="Optional invoice / receipt number"
                />
              </div>
            </div>
            <div class="modal-footer">
              <button type="button" class="btn btn-outline-secondary" @click="showModal = false">
                Cancel
              </button>
              <button type="submit" class="btn btn-primary" :disabled="saving">
                <span v-if="saving" class="spinner-border spinner-border-sm me-2"></span>Save Expense
              </button>
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
            <h5 class="modal-title">Expense Categories</h5>
            <button type="button" class="btn-close" @click="showCatsModal = false"></button>
          </div>
          <div class="modal-body">
            <div v-if="categoryError" class="alert alert-danger py-2 small" role="alert">
              <i class="bi bi-exclamation-triangle me-1"></i>{{ categoryError }}
            </div>
            <form class="d-flex gap-2 mb-3" @submit.prevent="addCategory">
              <input
                v-model="newCategoryName"
                class="form-control"
                type="text"
                placeholder="New category name…"
                aria-label="New category name"
              />
              <button type="submit" class="btn btn-outline-primary" :disabled="catSaving">
                <span v-if="catSaving" class="spinner-border spinner-border-sm me-1"></span>Add
              </button>
            </form>
            <div v-if="!categories.length" class="text-muted small">No categories yet.</div>
            <ul class="list-group">
              <li
                v-for="c in categories"
                :key="c.id"
                class="list-group-item d-flex justify-content-between align-items-center"
              >
                <div>
                  <span class="fw-semibold">{{ c.name }}</span>
                  <span class="text-muted small ms-2">{{ c.expenseCount }} expense(s)</span>
                </div>
                <button
                  class="btn btn-sm btn-outline-danger"
                  type="button"
                  title="Delete"
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
