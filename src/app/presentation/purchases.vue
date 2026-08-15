<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useCatalogStore } from "../../stores/catalog";
import { useSettingsStore } from "../../stores/settings";
import { useAuth } from "../../composables/useAuth";
import type { Supplier, SupplierInvoice, InvoiceLine } from "../../types";

const catalog = useCatalogStore();
const settings = useSettingsStore();
const auth = useAuth();

const suppliers = ref<Supplier[]>([]);
const invoices = ref<SupplierInvoice[]>([]);
const loading = ref(false);
const error = ref("");
const notice = ref("");

const showModal = ref(false);
const saving = ref(false);
const formError = ref("");
const form = ref({
  supplierId: null as number | null,
  date: new Date().toISOString().slice(0, 10),
  notes: "",
  lines: [] as InvoiceLine[],
});

function fmt(n: number): string {
  if (!settings.currency) return n.toFixed(2);
  return new Intl.NumberFormat(undefined, {
    style: "currency",
    currency: settings.currency,
    currencyDisplay: "narrowSymbol",
  }).format(n);
}

function lineSubtotal(line: InvoiceLine): number {
  return (line.qty || 0) * (line.costPrice || 0);
}

const invoiceTotal = computed(() =>
  form.value.lines.reduce((sum, l) => sum + lineSubtotal(l), 0)
);

const activeProducts = computed(() => catalog.products.filter((p) => p.is_active === 1));

async function load() {
  loading.value = true;
  error.value = "";
  try {
    const [s, i] = await Promise.all([
      invoke<Supplier[]>("list_suppliers"),
      invoke<SupplierInvoice[]>("list_supplier_invoices"),
    ]);
    suppliers.value = s;
    invoices.value = i;
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

function emptyLine(): InvoiceLine {
  return { productId: null, qty: 1, costPrice: 0 };
}

function openAdd() {
  formError.value = "";
  form.value = {
    supplierId: suppliers.value[0]?.id ?? null,
    date: new Date().toISOString().slice(0, 10),
    notes: "",
    lines: [emptyLine()],
  };
  showModal.value = true;
}

function addLine() {
  form.value.lines.push(emptyLine());
}

function removeLine(index: number) {
  form.value.lines.splice(index, 1);
}

function onProductChange(line: InvoiceLine) {
  const product = catalog.products.find((p) => p.id === line.productId);
  if (product) line.costPrice = product.cost_price;
}

function validate(): string {
  if (!form.value.supplierId) return "Select a supplier";
  const validLines = form.value.lines.filter((l) => l.productId != null);
  if (!validLines.length) return "Add at least one product line";
  for (const l of validLines) {
    if (!l.qty || l.qty <= 0) return "Quantities must be greater than zero";
    if (typeof l.costPrice !== "number" || isNaN(l.costPrice) || l.costPrice < 0)
      return "Enter a valid cost price (0 or more)";
  }
  return "";
}

function payload() {
  return {
    supplierId: form.value.supplierId,
    date: form.value.date,
    notes: form.value.notes || null,
    items: form.value.lines
      .filter((l) => l.productId != null)
      .map((l) => ({ productId: l.productId, qty: l.qty, costPrice: l.costPrice })),
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
    await invoke<number>("create_supplier_invoice", { input: payload() });
    showModal.value = false;
    notice.value = "Invoice recorded — stock updated";
    await Promise.all([load(), catalog.load()]);
  } catch (e) {
    formError.value = e instanceof Error ? e.message : String(e);
  } finally {
    saving.value = false;
  }
}

function statusBadge(status: string) {
  switch (status) {
    case "paid":
      return "text-bg-success";
    case "partial":
      return "text-bg-warning";
    default:
      return "text-bg-danger";
  }
}

onMounted(async () => {
  await Promise.allSettled([load(), catalog.load(), settings.load()]);
});
</script>

<template>
  <div>
    <div class="d-flex align-items-center justify-content-between mb-3">
      <h1 class="h4 mb-0">Purchases</h1>
      <button v-can="'expenses.manage'" class="btn btn-primary" type="button" @click="openAdd">
        <i class="bi bi-plus-lg me-1"></i>New Invoice
      </button>
    </div>

    <div v-if="error" class="alert alert-danger py-2 small" role="alert">
      <i class="bi bi-exclamation-triangle me-1"></i>{{ error }}
    </div>
    <div v-if="notice" class="alert alert-success py-2 small" role="alert">
      <i class="bi bi-check-circle me-1"></i>{{ notice }}
    </div>

    <div class="card">
      <div class="table-responsive">
        <table class="table align-middle mb-0">
          <thead>
            <tr>
              <th>Invoice</th>
              <th>Supplier</th>
              <th>Date</th>
              <th class="text-end">Total</th>
              <th class="text-end">Paid</th>
              <th class="text-end">Due</th>
              <th>Status</th>
            </tr>
          </thead>
          <tbody>
            <tr v-if="loading">
              <td colspan="7" class="text-center text-muted py-4">Loading…</td>
            </tr>
            <tr v-else-if="!invoices.length">
              <td colspan="7" class="text-center text-muted py-4">
                No incoming invoices yet — click "New Invoice" to record stock-in from a supplier
              </td>
            </tr>
            <tr v-for="inv in invoices" :key="inv.id">
              <td class="fw-semibold">{{ inv.invoice_no }}</td>
              <td>{{ inv.supplier_name }}</td>
              <td class="text-muted">{{ inv.date }}</td>
              <td class="text-end">{{ fmt(inv.total) }}</td>
              <td class="text-end">{{ fmt(inv.paid_amount) }}</td>
              <td class="text-end">
                <span v-if="inv.due_amount > 0" class="text-danger fw-semibold">
                  {{ fmt(inv.due_amount) }}
                </span>
                <span v-else class="text-muted">—</span>
              </td>
              <td>
                <span class="badge" :class="statusBadge(inv.status)">
                  {{ inv.status }}
                </span>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <div v-if="showModal" class="modal-backdrop show"></div>
    <div v-if="showModal" class="modal d-block" tabindex="-1">
      <div class="modal-dialog modal-lg modal-dialog-centered modal-dialog-scrollable">
        <div class="modal-content">
          <form @submit.prevent="save">
            <div class="modal-header">
              <h5 class="modal-title">New Incoming Invoice</h5>
              <button type="button" class="btn-close" @click="showModal = false"></button>
            </div>
            <div class="modal-body">
              <div v-if="formError" class="alert alert-danger py-2 small" role="alert">
                <i class="bi bi-exclamation-triangle me-1"></i>{{ formError }}
              </div>
              <div class="row g-3">
                <div class="col-md-6">
                  <label class="form-label" for="pi-supplier">Supplier *</label>
                  <select id="pi-supplier" v-model="form.supplierId" class="form-select">
                    <option :value="null">Select a supplier…</option>
                    <option v-for="s in suppliers" :key="s.id" :value="s.id">{{ s.name }}</option>
                  </select>
                </div>
                <div class="col-md-6">
                  <label class="form-label" for="pi-date">Date</label>
                  <input id="pi-date" v-model="form.date" class="form-control" type="date" />
                </div>
              </div>

              <div class="d-flex justify-content-between align-items-center mt-4 mb-2">
                <span class="fw-semibold small text-muted text-uppercase">Products</span>
                <button type="button" class="btn btn-sm btn-outline-primary" @click="addLine">
                  <i class="bi bi-plus-lg me-1"></i>Add line
                </button>
              </div>

              <div v-if="!form.lines.length" class="text-muted small py-2">
                No lines yet — click "Add line" to add products.
              </div>
              <div
                v-for="(line, index) in form.lines"
                :key="index"
                class="row g-2 align-items-center invoice-line"
              >
                <div class="col-6">
                  <select
                    class="form-select form-select-sm"
                    :value="line.productId ?? ''"
                    @change="
                      (e) => {
                        line.productId = (e.target as HTMLSelectElement).value
                          ? Number((e.target as HTMLSelectElement).value)
                          : null;
                        onProductChange(line);
                      }
                    "
                  >
                    <option value="">Select product…</option>
                    <option v-for="p in activeProducts" :key="p.id" :value="p.id">
                      {{ p.name }}
                    </option>
                  </select>
                </div>
                <div class="col-2">
                  <input
                    v-model.number="line.qty"
                    class="form-control form-control-sm text-end"
                    type="number"
                    step="0.5"
                    min="0"
                    :placeholder="catalog.products.find((p) => p.id === line.productId)?.unit ?? 'qty'"
                  />
                </div>
                <div class="col-2">
                  <input
                    v-model.number="line.costPrice"
                    class="form-control form-control-sm text-end"
                    type="number"
                    step="0.01"
                    min="0"
                    placeholder="cost"
                  />
                </div>
                <div class="col-2 d-flex justify-content-between align-items-center">
                  <span class="text-nowrap small fw-semibold">{{ fmt(lineSubtotal(line)) }}</span>
                  <button
                    type="button"
                    class="btn btn-sm btn-outline-danger p-1"
                    title="Remove line"
                    @click="removeLine(index)"
                  >
                    <i class="bi bi-x-lg"></i>
                  </button>
                </div>
              </div>

              <div class="d-flex justify-content-end mt-3 pt-2 border-top">
                <div class="d-flex align-items-center gap-3">
                  <span class="text-muted small">{{ form.lines.length }} line(s)</span>
                  <span class="fw-semibold">Total: {{ fmt(invoiceTotal) }}</span>
                </div>
              </div>

              <div class="mt-3">
                <label class="form-label" for="pi-notes">Notes</label>
                <textarea
                  id="pi-notes"
                  v-model="form.notes"
                  class="form-control"
                  rows="2"
                  placeholder="Optional reference or remarks…"
                ></textarea>
              </div>
            </div>
            <div class="modal-footer">
              <button type="button" class="btn btn-outline-secondary" @click="showModal = false">
                Cancel
              </button>
              <button type="submit" class="btn btn-primary" :disabled="saving">
                <span v-if="saving" class="spinner-border spinner-border-sm me-2"></span>Save Invoice
              </button>
            </div>
          </form>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.invoice-line {
  padding: 0.5rem;
  border: 1px solid var(--bs-border-color);
  border-radius: 0.5rem;
  margin-bottom: 0.5rem;
}
</style>
