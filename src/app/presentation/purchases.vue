<template>
  <div>
    <div class="d-flex align-items-center justify-content-between mb-3">
      <h1 class="h4 mb-0">{{ t("purchases.title") }}</h1>
      <button
        v-can="'expenses.manage'"
        class="btn btn-primary"
        type="button"
        @click="openAdd"
      >
        <i class="bi bi-plus-lg mx-1"></i>{{ t("purchases.newInvoice") }}
      </button>
    </div>

    <div class="card">
      <div class="p-2 border-bottom">
        <input
          v-model="search"
          class="form-control form-control-sm"
          type="search"
          :placeholder="t('purchases.searchPlaceholder')"
        />
      </div>
      <div class="table-responsive">
        <table class="table align-middle mb-0">
          <thead v-if="invoices.length">
            <tr>
              <th>{{ t("purchases.invoice") }}</th>
              <th>{{ t("purchases.supplier") }}</th>
              <th>{{ t("common.date") }}</th>
              <th class="text-end">{{ t("common.total") }}</th>
              <th class="text-end">{{ t("common.paid") }}</th>
              <th class="text-end">{{ t("common.due") }}</th>
              <th>{{ t("common.status") }}</th>
              <th class="text-end">{{ t("common.actions") }}</th>
            </tr>
          </thead>
          <tbody v-if="loading">
            <tr>
              <td colspan="8" class="text-center text-muted py-4">
                {{ t("common.loading") }}
              </td>
            </tr>
          </tbody>
          <tbody v-else-if="!invoices.length">
            <tr>
              <td colspan="8" class="p-0 border-0">
                <EmptyState
                  :image="emptyPurchases"
                  :message="t('purchases.noInvoices')"
                />
              </td>
            </tr>
          </tbody>
          <tbody v-for="inv in invoices" :key="inv.id">
            <tr>
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
                  {{ statusLabel(inv.status) }}
                </span>
              </td>
              <td class="text-end text-nowrap">
                <button
                  v-if="inv.due_amount > 0"
                  class="btn btn-sm btn-outline-success"
                  type="button"
                  :title="t('purchases.recordPaymentTitle')"
                  @click="openPay(inv)"
                >
                  <i class="bi bi-cash-coin mx-1"></i>{{ t("purchases.pay") }}
                </button>
                <span v-else class="text-muted small fst-italic">{{
                  t("purchases.settled")
                }}</span>
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

    <div v-if="showModal" class="modal-backdrop show"></div>
    <div v-if="showModal" class="modal d-block" tabindex="-1">
      <div
        class="modal-dialog modal-lg modal-dialog-centered modal-dialog-scrollable"
      >
        <div class="modal-content">
          <form @submit.prevent="save">
            <div class="modal-header">
              <h5 class="modal-title">{{ t("purchases.newInvoiceTitle") }}</h5>
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
              <div class="row g-3">
                <div class="col-md-6">
                  <label class="form-label" for="pi-supplier"
                    >{{ t("purchases.supplier") }} *</label
                  >
                  <select
                    id="pi-supplier"
                    v-model="form.supplierId"
                    class="form-select"
                    :class="{ 'is-invalid': errors.supplierId }"
                    @change="clearFieldError(errors, 'supplierId')"
                  >
                    <option :value="null">
                      {{ t("purchases.selectSupplier") }}
                    </option>
                    <option v-for="s in suppliers" :key="s.id" :value="s.id">
                      {{ s.name }}
                    </option>
                  </select>
                  <div class="invalid-feedback">{{ errors.supplierId }}</div>
                </div>
                <div class="col-md-6">
                  <label class="form-label" for="pi-date">{{
                    t("common.date")
                  }}</label>
                  <input
                    id="pi-date"
                    v-model="form.date"
                    class="form-control"
                    type="date"
                  />
                </div>
              </div>

              <div
                class="d-flex justify-content-between align-items-center mt-4 mb-2"
              >
                <span class="fw-semibold small text-muted text-uppercase">{{
                  t("purchases.products")
                }}</span>
                <button
                  type="button"
                  class="btn btn-sm btn-outline-primary"
                  @click="addLine"
                >
                  <i class="bi bi-plus-lg mx-1"></i>{{ t("purchases.addLine") }}
                </button>
              </div>

              <div v-if="!form.lines.length" class="text-muted small py-2">
                {{ t("purchases.noLines") }}
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
                    <option value="">{{ t("purchases.selectProduct") }}</option>
                    <option
                      v-for="p in activeProducts"
                      :key="p.id"
                      :value="p.id"
                    >
                      {{ p.name }}
                    </option>
                  </select>
                </div>
                <div class="col-2">
                  <input
                    v-model.number="line.qty"
                    class="form-control form-control-sm text-end"
                    type="number"
                    step="1"
                    min="0"
                    :placeholder="
                      catalog.products.find((p) => p.id === line.productId)
                        ?.unit ?? 'qty'
                    "
                  />
                </div>
                <div class="col-2">
                  <input
                    v-model.number="line.costPrice"
                    class="form-control form-control-sm text-end"
                    type="number"
                    step="1"
                    min="0"
                    :placeholder="t('products.cost')"
                  />
                </div>
                <div
                  class="col-2 d-flex justify-content-between align-items-center"
                >
                  <span class="text-nowrap small fw-semibold">{{
                    fmt(lineSubtotal(line))
                  }}</span>
                  <button
                    type="button"
                    class="btn btn-sm btn-outline-danger p-1"
                    :title="t('common.remove')"
                    @click="removeLine(index)"
                  >
                    <i class="bi bi-x-lg"></i>
                  </button>
                </div>
              </div>

              <div class="d-flex justify-content-end mt-3 pt-2 border-top">
                <div class="d-flex align-items-center gap-3">
                  <span class="text-muted small">
                    {{
                      t(
                        "purchases.lineCount",
                        { count: form.lines.length },
                        form.lines.length,
                      )
                    }}
                  </span>
                  <span class="fw-semibold">{{
                    t("purchases.totalLabel", { total: fmt(invoiceTotal) })
                  }}</span>
                </div>
              </div>

              <div class="mt-3">
                <label class="form-label" for="pi-notes">{{
                  t("common.notes")
                }}</label>
                <textarea
                  id="pi-notes"
                  v-model="form.notes"
                  class="form-control"
                  rows="2"
                  :placeholder="t('purchases.notesPlaceholder')"
                ></textarea>
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
                  {{ t("purchases.saveInvoice") }}
                </AsyncButton>
              </div>
            </div>
          </form>
        </div>
      </div>
    </div>

    <div v-if="payTarget" class="modal-backdrop show"></div>
    <div v-if="payTarget" class="modal d-block" tabindex="-1">
      <div class="modal-dialog modal-dialog-centered">
        <div class="modal-content">
          <form @submit.prevent="savePayment">
            <div class="modal-header">
              <h5 class="modal-title">
                {{
                  t("purchases.recordPaymentTitle2", {
                    invoiceNo: payTarget.invoice_no,
                  })
                }}
              </h5>
              <button
                type="button"
                class="btn-close"
                @click="payTarget = null"
              ></button>
            </div>
            <div class="modal-body">
              <div
                v-if="payError"
                class="alert alert-danger py-2 small"
                role="alert"
              >
                <i class="bi bi-exclamation-triangle mx-1"></i>{{ payError }}
              </div>
              <div class="d-flex gap-3 mb-3">
                <div class="flex-grow-1 card text-center p-2">
                  <div class="text-muted small text-uppercase">
                    {{ t("common.total") }}
                  </div>
                  <div class="fw-semibold">{{ fmt(payTarget.total) }}</div>
                </div>
                <div class="flex-grow-1 card text-center p-2">
                  <div class="text-muted small text-uppercase">
                    {{ t("common.paid") }}
                  </div>
                  <div class="fw-semibold">
                    {{ fmt(payTarget.paid_amount) }}
                  </div>
                </div>
                <div class="flex-grow-1 card text-center p-2">
                  <div class="text-muted small text-uppercase">
                    {{ t("purchases.outstanding") }}
                  </div>
                  <div class="fw-semibold text-danger">
                    {{ fmt(payTarget.due_amount) }}
                  </div>
                </div>
              </div>
              <div class="row g-3">
                <div class="col-md-6">
                  <label class="form-label" for="pay-amount">{{
                    t("purchases.amountRequired")
                  }}</label>
                  <input
                    id="pay-amount"
                    v-model.number="payForm.amount"
                    class="form-control"
                    :class="{ 'is-invalid': payErrors.amount }"
                    type="number"
                    step="0.01"
                    min="0.01"
                    :max="payTarget.due_amount"
                    autofocus
                    @input="clearFieldError(payErrors, 'amount')"
                  />
                  <div class="invalid-feedback">{{ payErrors.amount }}</div>
                </div>
                <div class="col-md-6">
                  <label class="form-label" for="pay-method">{{
                    t("common.method")
                  }}</label>
                  <select
                    id="pay-method"
                    v-model="payForm.method"
                    class="form-select"
                  >
                    <option value="cash">{{ t("purchases.cash") }}</option>
                    <option value="card">{{ t("purchases.card") }}</option>
                    <option value="bank">{{ t("purchases.bank") }}</option>
                  </select>
                </div>
              </div>
              <div class="mt-3">
                <label class="form-label" for="pay-date">{{
                  t("common.date")
                }}</label>
                <input
                  id="pay-date"
                  v-model="payForm.date"
                  class="form-control"
                  type="date"
                />
              </div>
              <div class="mt-3 mb-0">
                <label class="form-label" for="pay-notes">{{
                  t("common.notes")
                }}</label>
                <input
                  id="pay-notes"
                  v-model="payForm.notes"
                  class="form-control"
                  type="text"
                  :placeholder="t('purchases.notesPlaceholder')"
                />
              </div>

              <div v-if="payHistory.length" class="mt-4">
                <div class="fw-semibold small text-muted text-uppercase mb-2">
                  {{ t("purchases.paymentHistory") }}
                </div>
                <table class="table table-sm align-middle mb-0">
                  <thead>
                    <tr>
                      <th>{{ t("common.date") }}</th>
                      <th>{{ t("common.method") }}</th>
                      <th>{{ t("common.notes") }}</th>
                      <th class="text-end">{{ t("common.amount") }}</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr v-for="p in payHistory" :key="p.id">
                      <td class="text-muted">{{ p.date }}</td>
                      <td class="text-capitalize">
                        {{ paymentMethodLabel(p.method) }}
                      </td>
                      <td class="text-muted">{{ p.notes ?? "—" }}</td>
                      <td class="text-end fw-semibold">{{ fmt(p.amount) }}</td>
                    </tr>
                  </tbody>
                </table>
              </div>
              <div
                class="d-flex justify-content-end gap-2 mt-3 pt-3 border-top"
              >
                <button
                  type="button"
                  class="btn btn-outline-secondary"
                  @click="payTarget = null"
                >
                  {{ t("common.close") }}
                </button>
                <AsyncButton
                  type="submit"
                  variant="success"
                  :loading="paySaving"
                  :disabled="!canPay"
                >
                  {{ t("purchases.recordPaymentBtn") }}
                </AsyncButton>
              </div>
            </div>
          </form>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useI18n } from "vue-i18n";
import EmptyState from "../../components/EmptyState.vue";
import AsyncButton from "../../components/AsyncButton.vue";
import { clearFieldError, useFormGuard } from "../../composables/useFormGuard";
import { usePagedList } from "../../composables/usePagedList";
import { useToast } from "../../composables/useToast";
import { useCatalogStore } from "../../stores/catalog";
import { useSettingsStore } from "../../stores/settings";
import { useAuth } from "../../composables/useAuth";
import type {
  Supplier,
  SupplierInvoice,
  InvoiceLine,
  SupplierPayment,
} from "../../types";
import emptyPurchases from "../../assets/empty/purchases.svg";
import { formatMoney } from "../../lib/currency";

const catalog = useCatalogStore();
const settings = useSettingsStore();
const auth = useAuth();
const toast = useToast();
const { t } = useI18n();

// Full list: the "new invoice" supplier dropdown is a small lookup.
const suppliers = ref<Supplier[]>([]);
const search = ref("");

const {
  items: invoices,
  loading,
  loadingMore,
  hasMore,
  reload: reloadInvoices,
  loadMore,
} = usePagedList<SupplierInvoice>(
  (limit, offset) =>
    invoke<SupplierInvoice[]>("list_supplier_invoices", {
      search: search.value.trim() || null,
      limit,
      offset,
    }),
  [search],
  (e) => toast.error(e instanceof Error ? e.message : String(e)),
);

async function loadSuppliers() {
  try {
    suppliers.value = await invoke<Supplier[]>("list_suppliers");
  } catch (e) {
    toast.error(e instanceof Error ? e.message : String(e));
  }
}

async function refresh() {
  await Promise.all([reloadInvoices(), loadSuppliers()]);
}

const showModal = ref(false);
const saving = ref(false);
const formError = ref("");
const form = ref({
  supplierId: null as number | null,
  date: new Date().toISOString().slice(0, 10),
  notes: "",
  lines: [] as InvoiceLine[],
});
const errors = reactive<Record<string, string>>({});
const guard = useFormGuard(form);
const canSave = computed(() => guard.isDirty.value && !saving.value);

function resetErrors() {
  for (const key of Object.keys(errors)) delete errors[key];
}

const payTarget = ref<SupplierInvoice | null>(null);
const payHistory = ref<SupplierPayment[]>([]);
const paySaving = ref(false);
const payError = ref("");
const payForm = ref({ amount: 0, method: "cash", date: "", notes: "" });
const payErrors = reactive<Record<string, string>>({});
const payGuard = useFormGuard(payForm);
const canPay = computed(() => !paySaving.value);

function fmt(n: number): string {
  return formatMoney(n);
}

function statusLabel(status: string): string {
  switch (status) {
    case "paid":
      return t("purchases.settled");
    case "partial":
      return t("expenses.partial");
    default:
      return t("expenses.unpaid");
  }
}

function paymentMethodLabel(method: string): string {
  return t("purchases." + method);
}

function lineSubtotal(line: InvoiceLine): number {
  return (line.qty || 0) * (line.costPrice || 0);
}

const invoiceTotal = computed(() =>
  form.value.lines.reduce((sum, l) => sum + lineSubtotal(l), 0),
);

const activeProducts = computed(() =>
  catalog.products.filter((p) => p.is_active === 1),
);

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
  resetErrors();
  guard.capture();
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

function validate(): boolean {
  resetErrors();
  let ok = !!form.value.supplierId;
  if (!ok) {
    errors.supplierId = t("purchases.selectSupplierErr");
  }
  const validLines = form.value.lines.filter((l) => l.productId != null);
  if (!validLines.length) {
    formError.value = "";
    toast.error(t("purchases.atLeastOneLine"));
    return false;
  }
  for (const l of validLines) {
    if (!l.qty || l.qty <= 0) {
      formError.value = t("purchases.qtyPositive");
      return false;
    }
    if (
      typeof l.costPrice !== "number" ||
      isNaN(l.costPrice) ||
      l.costPrice < 0
    ) {
      formError.value = t("purchases.costValid");
      return false;
    }
  }
  return ok;
}

function payload() {
  return {
    supplierId: form.value.supplierId,
    date: form.value.date,
    notes: form.value.notes || null,
    items: form.value.lines
      .filter((l) => l.productId != null)
      .map((l) => ({
        productId: l.productId,
        qty: l.qty,
        costPrice: l.costPrice,
      })),
    userId: auth.user?.id ?? null,
  };
}

async function save() {
  formError.value = "";
  if (!validate()) {
    if (Object.keys(errors).length) toast.error(t("common.fixErrors"));
    return;
  }
  saving.value = true;
  try {
    await invoke<number>("create_supplier_invoice", { input: payload() });
    guard.markSaved();
    showModal.value = false;
    toast.success(t("purchases.invoiceRecorded"));
    await Promise.all([refresh(), catalog.load()]);
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

async function openPay(inv: SupplierInvoice) {
  payError.value = "";
  delete payErrors.amount;
  payHistory.value = [];
  payTarget.value = inv;
  payForm.value = {
    amount: Number(inv.due_amount.toFixed(2)),
    method: "cash",
    date: new Date().toISOString().slice(0, 10),
    notes: "",
  };
  payGuard.capture();
  try {
    payHistory.value = await invoke<SupplierPayment[]>(
      "list_supplier_payments",
      {
        invoiceId: inv.id,
      },
    );
  } catch (e) {
    payError.value = e instanceof Error ? e.message : String(e);
  }
}

async function savePayment() {
  const inv = payTarget.value;
  if (!inv) return;
  payError.value = "";
  const amount = Number(payForm.value.amount);
  const outstanding = Number(inv.due_amount.toFixed(2));
  if (isNaN(amount) || amount < 0.01) {
    payErrors.amount = t("purchases.paymentAmount");
    toast.error(t("common.fixErrors"));
    return;
  }
  const rounded = Math.round(amount * 100) / 100;
  if (rounded > outstanding) {
    payErrors.amount = t("purchases.paymentExceeds", {
      outstanding: fmt(outstanding),
    });
    toast.error(t("common.fixErrors"));
    return;
  }
  paySaving.value = true;
  try {
    await invoke("add_supplier_payment", {
      input: {
        invoiceId: inv.id,
        amount: rounded,
        method: payForm.value.method,
        date: payForm.value.date,
        notes: payForm.value.notes || null,
        userId: auth.user?.id ?? null,
      },
    });
    payGuard.markSaved();
    payTarget.value = null;
    toast.success(
      t("purchases.paymentRecorded", {
        invoiceNo: inv.invoice_no,
      }),
    );
    await reloadInvoices();
  } catch (e) {
    payError.value = e instanceof Error ? e.message : String(e);
  } finally {
    paySaving.value = false;
  }
}

onMounted(async () => {
  await Promise.allSettled([
    reloadInvoices(),
    loadSuppliers(),
    catalog.load(),
    settings.load(),
  ]);
});
</script>

<style scoped>
.invoice-line {
  padding: 0.5rem;
  border: 1px solid var(--bs-border-color);
  border-radius: 0.5rem;
  margin-bottom: 0.5rem;
}
</style>
