<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useSettingsStore } from "../../stores/settings";
import { useAuth } from "../../composables/useAuth";
import type { ExpenseCategory, OutgoingExpense } from "../../types";

const settings = useSettingsStore();
const auth = useAuth();

const expenses = ref<OutgoingExpense[]>([]);
const categories = ref<ExpenseCategory[]>([]);
const loading = ref(false);
const error = ref("");
const notice = ref("");

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

const totalSpent = computed(() => expenses.value.reduce((sum, e) => sum + e.amount, 0));

function fmt(n: number): string {
  if (!settings.currency) return n.toFixed(2);
  return new Intl.NumberFormat(undefined, {
    style: "currency",
    currency: settings.currency,
    currencyDisplay: "narrowSymbol",
  }).format(n);
}

async function load() {
  loading.value = true;
  error.value = "";
  try {
    const [e, c] = await Promise.all([
      invoke<OutgoingExpense[]>("list_expenses_out"),
      invoke<ExpenseCategory[]>("list_expense_categories"),
    ]);
    expenses.value = e;
    categories.value = c;
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

    <div class="card mb-3">
      <div class="card-body d-flex justify-content-between align-items-center py-2">
        <span class="text-muted small">Total recorded ({{ expenses.length }} expense(s))</span>
        <span class="fs-5 fw-semibold">{{ fmt(totalSpent) }}</span>
      </div>
    </div>

    <div class="card">
      <div class="table-responsive">
        <table class="table align-middle mb-0">
          <thead>
            <tr>
              <th>Date</th>
              <th>Description</th>
              <th>Category</th>
              <th>Reference</th>
              <th>Recorded by</th>
              <th class="text-end">Amount</th>
            </tr>
          </thead>
          <tbody>
            <tr v-if="loading">
              <td colspan="6" class="text-center text-muted py-4">Loading…</td>
            </tr>
            <tr v-else-if="!expenses.length">
              <td colspan="6" class="text-center text-muted py-4">
                No expenses recorded yet — click "New Expense" to record money out
              </td>
            </tr>
            <tr v-for="e in expenses" :key="e.id">
              <td class="text-muted">{{ e.date }}</td>
              <td class="fw-semibold">{{ e.description ?? "—" }}</td>
              <td>
                <span v-if="e.category_name" class="badge text-bg-light border">
                  {{ e.category_name }}
                </span>
                <span v-else class="text-muted">—</span>
              </td>
              <td class="text-muted">{{ e.reference_no ?? "—" }}</td>
              <td class="text-muted">{{ e.user_name ?? "—" }}</td>
              <td class="text-end fw-semibold text-danger">{{ fmt(e.amount) }}</td>
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
