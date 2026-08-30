<template>
  <div>
    <div class="d-flex align-items-center justify-content-between mb-3">
      <h1 class="h4 mb-0">{{ t("customers.title") }}</h1>
      <button class="btn btn-primary" type="button" @click="openAdd">
        <i class="bi bi-person-plus mx-1"></i>{{ t("customers.addCustomer") }}
      </button>
    </div>

    <div class="row g-3 mb-3">
      <div class="col-md-4">
        <div class="kpi-card">
          <div class="kpi-icon"><i class="bi bi-people"></i></div>
          <div>
            <div class="kpi-label">{{ t("customers.totalCustomers") }}</div>
            <div class="kpi-value">{{ totalCustomers }}</div>
          </div>
        </div>
      </div>
      <div class="col-md-4">
        <div class="kpi-card">
          <div class="kpi-icon"><i class="bi bi-person-exclamation"></i></div>
          <div>
            <div class="kpi-label">{{ t("customers.debtors") }}</div>
            <div class="kpi-value">{{ debtorsCount }}</div>
          </div>
        </div>
      </div>
      <div class="col-md-4">
        <div class="kpi-card">
          <div class="kpi-icon"><i class="bi bi-cash-coin"></i></div>
          <div>
            <div class="kpi-label">{{ t("customers.totalDebt") }}</div>
            <div class="kpi-value" :class="totalDebt > 0 ? 'text-danger' : ''">
              {{ fmt(totalDebt) }}
            </div>
          </div>
        </div>
      </div>
    </div>

    <div v-if="debtors.length" class="card mb-3">
      <div
        class="card-header d-flex align-items-center justify-content-between py-2"
      >
        <span class="fw-semibold small text-uppercase text-muted">
          <i class="bi bi-collection mx-1"></i
          >{{ t("customers.debtCollection") }}
        </span>
        <span>
          <span class="badge text-bg-danger rounded-pill mx-2">{{
            debtorsCount
          }}</span>
          <strong class="text-danger">{{ fmt(totalDebt) }}</strong>
        </span>
      </div>
      <div class="debtor-list">
        <table class="table table-sm align-middle mb-0">
          <tbody>
            <tr v-for="c in debtors" :key="c.id">
              <td class="fw-semibold">{{ c.name }}</td>
              <td class="text-muted" style="direction: ltr">
                {{ c.phone ?? "—" }}
              </td>
              <td class="text-start">
                <span class="badge text-bg-danger rounded-pill">{{
                  fmt(c.balance)
                }}</span>
              </td>
              <td class="text-start text-nowrap">
                <button
                  class="btn btn-sm btn-outline-secondary"
                  type="button"
                  :title="t('customers.viewProfileTitle')"
                  @click="openDetail(c)"
                >
                  <i class="bi bi-eye"></i>
                </button>
                <button
                  class="btn btn-sm btn-outline-success ms-1"
                  type="button"
                  :title="t('customers.recordPayment')"
                  @click="quickCollect(c)"
                >
                  <i class="bi bi-cash-coin"></i>
                </button>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <div class="card">
      <div class="p-2 border-bottom">
        <input
          v-model="search"
          class="form-control form-control-sm"
          type="search"
          :placeholder="t('customers.searchPlaceholder')"
        />
      </div>
      <div class="table-responsive">
        <table class="table align-middle mb-0">
          <thead v-if="customers.length">
            <tr>
              <th>{{ t("common.name") }}</th>
              <th>{{ t("customers.phone") }}</th>
              <th>{{ t("suppliers.email") }}</th>
              <th>{{ t("suppliers.address") }}</th>
              <th class="text-start">{{ t("customers.balance") }}</th>
              <th class="text-start">{{ t("customers.memberSince") }}</th>
              <th class="text-start">{{ t("common.actions") }}</th>
            </tr>
          </thead>
          <tbody v-if="loading">
            <tr>
              <td colspan="7" class="py-3">
                <div class="skeleton mb-2" style="height: 0.8rem"></div>
                <div
                  class="skeleton mb-2"
                  style="width: 86%; height: 0.8rem"
                ></div>
                <div class="skeleton" style="width: 92%; height: 0.8rem"></div>
              </td>
            </tr>
          </tbody>
          <tbody v-else-if="!customers.length">
            <tr>
              <td colspan="7" class="p-0 border-0">
                <EmptyState
                  :image="emptyCustomers"
                  :message="
                    search
                      ? t('customers.noMatching')
                      : t('customers.noCustomers')
                  "
                />
              </td>
            </tr>
          </tbody>
          <tbody v-for="c in customers" :key="c.id">
            <tr>
              <td>
                <div class="fw-semibold">{{ c.name }}</div>
                <div v-if="c.notes" class="text-muted text-xs">
                  {{ c.notes }}
                </div>
              </td>
              <td class="text-muted" style="direction: ltr">
                {{ c.phone ?? "—" }}
              </td>
              <td class="text-muted">{{ c.email ?? "—" }}</td>
              <td class="text-muted">{{ c.address ?? "—" }}</td>
              <td class="text-start">
                <span
                  v-if="c.balance > 0.005"
                  class="badge text-bg-danger rounded-pill"
                >
                  {{ fmt(c.balance) }}
                </span>
                <span v-else class="text-muted">—</span>
              </td>
              <td class="text-start text-muted">{{ dateLabel(c.created_at) }}</td>
              <td class="text-start text-nowrap">
                <button
                  class="btn btn-sm btn-outline-secondary mx-1"
                  type="button"
                  :title="t('customers.viewProfileTitle')"
                  @click="openDetail(c)"
                >
                  <i class="bi bi-eye"></i>
                </button>
                <button
                  class="btn btn-sm btn-outline-primary mx-1"
                  type="button"
                  :title="t('common.edit')"
                  @click="openEdit(c)"
                >
                  <i class="bi bi-pencil-square"></i>
                </button>
                <button
                  class="btn btn-sm btn-outline-danger"
                  type="button"
                  :title="t('common.delete')"
                  @click="remove(c)"
                >
                  <i class="bi bi-trash"></i>
                </button>
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
          <form novalidate @submit.prevent="save">
            <div class="modal-header">
              <h5 class="modal-title">
                {{
                  editingId == null
                    ? t("customers.addTitle")
                    : t("customers.editTitle", { name: form.name })
                }}
              </h5>
              <button
                type="button"
                class="btn-close"
                @click="showModal = false"
              ></button>
            </div>
            <div class="modal-body">
              <div class="mb-3">
                <label class="form-label" for="c-name">{{
                  t("customers.nameRequired")
                }}</label>
                <input
                  id="c-name"
                  v-model="form.name"
                  class="form-control"
                  :class="{ 'is-invalid': !!fieldErrors.name }"
                  type="text"
                  autofocus
                />
                <div v-if="fieldErrors.name" class="invalid-feedback d-block">
                  {{ fieldErrors.name }}
                </div>
              </div>
              <div class="row g-3 mb-3">
                <div class="col-md-6">
                  <label class="form-label" for="c-phone"
                    >{{ t("customers.phone") }}
                    {{ t("common.optional") }}</label
                  >
                  <input
                    id="c-phone"
                    v-model="form.phone"
                    class="form-control"
                    :class="{ 'is-invalid': !!fieldErrors.phone }"
                    type="tel"
                    style="direction: ltr"
                  />
                  <div
                    v-if="fieldErrors.phone"
                    class="invalid-feedback d-block"
                  >
                    {{ fieldErrors.phone }}
                  </div>
                </div>
                <div class="col-md-6">
                  <label class="form-label" for="c-email"
                    >{{ t("suppliers.email") }}
                    {{ t("common.optional") }}</label
                  >
                  <input
                    id="c-email"
                    v-model="form.email"
                    class="form-control"
                    :class="{ 'is-invalid': !!fieldErrors.email }"
                    type="email"
                  />
                  <div
                    v-if="fieldErrors.email"
                    class="invalid-feedback d-block"
                  >
                    {{ fieldErrors.email }}
                  </div>
                </div>
              </div>
              <div class="mb-3">
                <label class="form-label" for="c-address"
                  >{{ t("suppliers.address") }}
                  {{ t("common.optional") }}</label
                >
                <textarea
                  id="c-address"
                  v-model="form.address"
                  class="form-control"
                  rows="2"
                ></textarea>
              </div>
              <div class="mb-0">
                <label class="form-label" for="c-notes"
                  >{{ t("common.notes") }} {{ t("common.optional") }}</label
                >
                <textarea
                  id="c-notes"
                  v-model="form.notes"
                  class="form-control"
                  rows="2"
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
                  {{ editingId == null ? t("common.add") : t("common.save") }}
                </AsyncButton>
              </div>
            </div>
          </form>
        </div>
      </div>
    </div>

    <div v-if="detail || detailLoading" class="modal-backdrop show"></div>
    <div v-if="detail || detailLoading" class="modal d-block" tabindex="-1">
      <div
        class="modal-dialog modal-lg modal-dialog-centered modal-dialog-scrollable"
      >
        <div class="modal-content">
          <div class="modal-header">
            <h5 class="modal-title">
              {{ t("customers.detailTitle", { name: detail?.name }) }}
            </h5>
            <button
              type="button"
              class="btn-close"
              @click="detail = null"
            ></button>
          </div>
          <div class="modal-body">
            <div v-if="detailLoading" class="text-center text-muted py-4">
              {{ t("common.loading") }}
            </div>
            <template v-else-if="detail">
              <div class="row g-3 mb-3">
                <div class="col-md-6">
                  <div class="detail-item">
                    <span class="detail-label">{{ t("customers.phone") }}</span>
                    <span style="direction: ltr">{{
                      detail.phone ?? "—"
                    }}</span>
                  </div>
                  <div class="detail-item">
                    <span class="detail-label">{{ t("suppliers.email") }}</span>
                    <span>{{ detail.email ?? "—" }}</span>
                  </div>
                </div>
                <div class="col-md-6">
                  <div class="detail-item">
                    <span class="detail-label">{{
                      t("suppliers.address")
                    }}</span>
                    <span>{{ detail.address ?? "—" }}</span>
                  </div>
                  <div class="detail-item">
                    <span class="detail-label">{{
                      t("customers.memberSince")
                    }}</span>
                    <span>{{ dateLabel(detail.created_at) }}</span>
                  </div>
                </div>
              </div>

              <div class="row g-3 mb-3">
                <div class="col">
                  <div class="card text-center p-3">
                    <div class="text-muted small text-uppercase">
                      {{ t("customers.currentBalance") }}
                    </div>
                    <div
                      class="fs-5 fw-semibold"
                      :class="detail.balance > 0 ? 'text-danger' : ''"
                    >
                      {{ fmt(detail.balance) }}
                    </div>
                  </div>
                </div>
                <div class="col">
                  <div class="card text-center p-3">
                    <div class="text-muted small text-uppercase">
                      {{ t("customers.purchases") }}
                    </div>
                    <div class="fs-5 fw-semibold">{{ kpis.count }}</div>
                  </div>
                </div>
                <div class="col">
                  <div class="card text-center p-3">
                    <div class="text-muted small text-uppercase">
                      {{ t("customers.lifetimePurchases") }}
                    </div>
                    <div class="fs-5 fw-semibold">{{ fmt(kpis.lifetime) }}</div>
                  </div>
                </div>
              </div>

              <div class="fw-semibold small text-muted text-uppercase mb-2">
                {{ t("customers.purchaseHistory") }}
              </div>
              <div v-if="!detail.sales.length" class="text-muted small py-2">
                {{ t("customers.noSalesYet") }}
              </div>
              <div v-else class="table-responsive mb-3">
                <table class="table table-sm align-middle mb-0">
                  <thead>
                    <tr>
                      <th>{{ t("sales.saleNo") }}</th>
                      <th>{{ t("common.date") }}</th>
                      <th class="text-start">{{ t("common.total") }}</th>
                      <th class="text-start">{{ t("common.paid") }}</th>
                      <th>{{ t("common.status") }}</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr v-for="s in detail.sales" :key="s.id">
                      <td class="fw-semibold">{{ s.sale_no }}</td>
                      <td class="text-muted">{{ dateLabel(s.created_at) }}</td>
                      <td class="text-start">{{ fmt(s.net_total) }}</td>
                      <td class="text-start">{{ fmt(s.paid_amount) }}</td>
                      <td>
                        <span
                          class="badge"
                          :class="
                            s.status === 'completed'
                              ? 'text-bg-success'
                              : 'text-bg-secondary'
                          "
                        >
                          {{
                            s.status === "completed"
                              ? t("customers.statusCompleted")
                              : t("customers.statusVoided")
                          }}
                        </span>
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>

              <div class="fw-semibold small text-muted text-uppercase mb-2">
                {{ t("customers.accountLedger") }}
              </div>
              <div v-if="!detail.ledger.length" class="text-muted small py-2">
                {{ t("customers.noLedgerEntries") }}
              </div>
              <div v-else class="table-responsive">
                <table class="table table-sm align-middle mb-0">
                  <thead>
                    <tr>
                      <th>{{ t("common.date") }}</th>
                      <th>{{ t("common.type") }}</th>
                      <th class="text-start">{{ t("common.amount") }}</th>
                      <th class="text-start">
                        {{ t("customers.balanceAfter") }}
                      </th>
                      <th>{{ t("common.notes") }}</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr v-for="e in detail.ledger" :key="e.id">
                      <td class="text-muted">{{ dateLabel(e.created_at) }}</td>
                      <td>
                        <span class="badge" :class="ledgerBadge(e.type)">
                          {{ ledgerLabel(e.type) }}
                        </span>
                      </td>
                      <td class="text-start" :class="ledgerClass(e.type)">
                        {{ ledgerSign(e.type) }}{{ fmt(e.amount) }}
                      </td>
                      <td class="text-start fw-semibold">
                        {{ fmt(e.balance_after) }}
                      </td>
                      <td class="text-muted">{{ e.notes ?? "—" }}</td>
                    </tr>
                  </tbody>
                </table>
              </div>
            </template>
            <div
              v-if="detail && !detailLoading"
              class="d-flex justify-content-end gap-2 mt-3 pt-3 border-top"
            >
              <button
                class="btn btn-success"
                type="button"
                :disabled="detail.balance <= 0.005"
                :title="
                  detail.balance <= 0.005 ? t('customers.noOutstanding') : ''
                "
                @click="openPayModal"
              >
                <i class="bi bi-cash-coin mx-1"></i
                >{{ t("customers.recordPayment") }}
              </button>
            </div>
          </div>
        </div>
      </div>

      <div v-if="showPayModal" class="modal-backdrop show"></div>
      <div v-if="showPayModal" class="modal d-block" tabindex="-1">
        <div class="modal-dialog modal-dialog-centered">
          <div class="modal-content">
            <form novalidate @submit.prevent="confirmPayment">
              <div class="modal-header">
                <h5 class="modal-title">{{ t("customers.recordPayment") }}</h5>
                <button
                  type="button"
                  class="btn-close"
                  @click="showPayModal = false"
                ></button>
              </div>
              <div class="modal-body">
                <p class="fw-semibold mb-1">{{ detail?.name }}</p>
                <p class="mb-3">
                  <span class="text-muted small mx-2"
                    >{{ t("customers.currentBalance") }}:</span
                  >
                  <strong class="text-danger">{{
                    fmt(detail?.balance ?? 0)
                  }}</strong>
                </p>
                <div
                  v-if="payError"
                  class="alert alert-danger py-2 small"
                  role="alert"
                >
                  <i class="bi bi-exclamation-triangle mx-1"></i>{{ payError }}
                </div>
                <div class="mb-3">
                  <label class="form-label" for="pay-amount">{{
                    t("customers.paymentAmountLabel")
                  }}</label>
                  <div class="input-group">
                    <input
                      id="pay-amount"
                      v-model.number="payAmount"
                      class="form-control"
                      :class="{ 'is-invalid': !!payError }"
                      type="number"
                      min="0"
                      step="1"
                      @input="payError = ''"
                    />
                    <button
                      class="btn btn-outline-secondary"
                      type="button"
                      @click="
                        payAmount = Number((detail?.balance ?? 0).toFixed(2))
                      "
                    >
                      {{ t("customers.payFullBalance") }}
                    </button>
                  </div>
                  <div v-if="payError" class="invalid-feedback d-block">
                    {{ payError }}
                  </div>
                </div>
                <div class="mb-0">
                  <label class="form-label" for="pay-notes"
                    >{{ t("common.notes") }} {{ t("common.optional") }}</label
                  >
                  <textarea
                    id="pay-notes"
                    v-model="payNotes"
                    class="form-control"
                    rows="2"
                  ></textarea>
                </div>
                <div
                  class="d-flex justify-content-end gap-2 mt-3 pt-3 border-top"
                >
                  <button
                    type="button"
                    class="btn btn-outline-secondary"
                    @click="showPayModal = false"
                  >
                    {{ t("common.cancel") }}
                  </button>
                  <AsyncButton
                    type="submit"
                    variant="success"
                    :loading="paySaving"
                    :disabled="!canPay"
                  >
                    {{ t("customers.recordPayment") }}
                  </AsyncButton>
                </div>
              </div>
            </form>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useI18n } from "vue-i18n";
import EmptyState from "../../components/EmptyState.vue";
import AsyncButton from "../../components/AsyncButton.vue";
import Paginator from "../../components/Paginator.vue";
import { useFormGuard } from "../../composables/useFormGuard";
import { usePagedList } from "../../composables/usePagedList";
import { useToast } from "../../composables/useToast";
import { useConfirm } from "../../composables/useConfirm";
import { useAuthStore } from "../../stores/auth";
import { useSettingsStore } from "../../stores/settings";
import { execute, insert, select, selectOne } from "../../lib/db";
import { formatMoney } from "../../lib/currency";
import type { Customer, CustomerLedgerRow, CustomerSaleRow } from "../../types";
import emptyCustomers from "../../assets/empty/customers.svg";

const settings = useSettingsStore();
const auth = useAuthStore();
const toast = useToast();
const { confirmDialog } = useConfirm();
const { t, locale } = useI18n();

// Global stats live in their own queries so the customer list can paginate
// without losing the KPI cards or the debt-collection panel.
const search = ref("");
const totalCustomers = ref(0);
const debtorsCount = ref(0);
const totalDebt = ref(0);
const debtors = ref<Customer[]>([]);

async function fetchStats() {
  const agg = await selectOne<{
    total: number;
    debtors_count: number | null;
    total_debt: number | null;
  }>(
    "SELECT COUNT(*) AS total, SUM(CASE WHEN balance > 0.005 THEN 1 ELSE 0 END) AS debtors_count, SUM(MAX(balance, 0)) AS total_debt FROM customers",
  );
  totalCustomers.value = agg?.total ?? 0;
  debtorsCount.value = agg?.debtors_count ?? 0;
  totalDebt.value = agg?.total_debt ?? 0;
  debtors.value = await select<Customer>(
    "SELECT id, name, phone, email, address, balance, notes, created_at FROM customers WHERE balance > 0.005 ORDER BY balance DESC LIMIT 50",
  );
}

const {
  items: customers,
  loading,
  page,
  size,
  totalItems,
  goToPage,
  reload: load,
} = usePagedList<Customer>(
  async (limit, offset) => {
    const p = `%${search.value.trim()}%`;
    const where =
      "WHERE name LIKE ? OR IFNULL(phone,'') LIKE ? OR IFNULL(email,'') LIKE ? OR IFNULL(address,'') LIKE ? OR IFNULL(notes,'') LIKE ?";
    const [rows, countRows] = await Promise.all([
      select<Customer>(
        `SELECT id, name, phone, email, address, balance, notes, created_at FROM customers ${where} ORDER BY name COLLATE NOCASE LIMIT ? OFFSET ?`,
        [p, p, p, p, p, limit, offset],
      ),
      select<{ total: number }>(
        `SELECT COUNT(*) AS total FROM customers ${where}`,
        [p, p, p, p, p],
      ),
      fetchStats(),
    ]);
    return { items: rows, total: Number(countRows[0]?.total ?? 0) };
  },
  [search],
  (e) => toast.error(e instanceof Error ? e.message : String(e)),
);

const showModal = ref(false);
const saving = ref(false);
const editingId = ref<number | null>(null);
const fieldErrors = ref<{ name?: string; phone?: string; email?: string }>({});
const form = ref({
  name: "",
  phone: "",
  email: "",
  address: "",
  notes: "",
});
const guard = useFormGuard(form);
const canSave = computed(() => guard.isDirty.value && !saving.value);

interface CustomerDetail extends Customer {
  sales: CustomerSaleRow[];
  ledger: CustomerLedgerRow[];
}

const detail = ref<CustomerDetail | null>(null);
const detailLoading = ref(false);

const showPayModal = ref(false);
const payAmount = ref(0);
const payNotes = ref("");
const paySaving = ref(false);
const payError = ref("");
const payForm = computed(() => ({
  amount: payAmount.value,
  notes: payNotes.value,
}));
const payGuard = useFormGuard(payForm);
const canPay = computed(() => payGuard.isDirty.value && !paySaving.value);

function openPayModal() {
  const d = detail.value;
  if (!d || d.balance <= 0.005) return;
  payError.value = "";
  payNotes.value = "";
  payAmount.value = 0;
  showPayModal.value = true;
  payGuard.capture();
}

async function confirmPayment() {
  const d = detail.value;
  if (!d) return;
  const amount = Number(payAmount.value);
  if (!Number.isFinite(amount) || amount <= 0) {
    payError.value = t("customers.amountRequired");
    toast.error(t("common.fixErrors"));
    return;
  }
  if (amount > d.balance + 0.005) {
    payError.value = t("customers.paymentTooLarge");
    toast.error(t("common.fixErrors"));
    return;
  }
  payError.value = "";
  paySaving.value = true;
  try {
    const newBalance = await invoke<number>("record_customer_payment", {
      customerId: d.id,
      amount,
      notes: payNotes.value.trim() || null,
      userId: auth.user?.id ?? null,
    });
    payGuard.markSaved();
    toast.success(t("customers.paymentRecorded", { amount: fmt(amount) }));
    showPayModal.value = false;
    await load();
    await openDetail({ ...d, balance: newBalance });
  } catch (e) {
    payError.value = String(e);
  } finally {
    paySaving.value = false;
  }
}

const kpis = computed(() => {
  const sales = detail.value?.sales ?? [];
  const completed = sales.filter((s) => s.status === "completed");
  return {
    count: completed.length,
    lifetime: completed.reduce((sum, s) => sum + s.net_total, 0),
  };
});

async function quickCollect(c: Customer) {
  await openDetail(c);
  openPayModal();
}

function fmt(n: number): string {
  return formatMoney(n);
}

function dateLabel(iso: string): string {
  const d = new Date(iso + (iso.includes("T") ? "" : "Z"));
  return Number.isNaN(d.getTime()) ? iso : d.toLocaleDateString(locale.value);
}

async function openDetail(c: Customer) {
  detail.value = null;
  detailLoading.value = true;
  try {
    const [sales, ledger] = await Promise.all([
      select<CustomerSaleRow>(
        "SELECT id, sale_no, created_at, total - COALESCE((SELECT SUM(amount) FROM refunds WHERE sale_id = sales.id), 0) AS net_total, paid_amount, status FROM sales WHERE customer_id = ? ORDER BY id DESC LIMIT 50",
        [c.id],
      ),
      select<CustomerLedgerRow>(
        "SELECT id, type, amount, balance_after, notes, created_at FROM customer_ledger WHERE customer_id = ? ORDER BY id DESC LIMIT 100",
        [c.id],
      ),
    ]);
    detail.value = { ...c, sales, ledger };
  } catch (e) {
    toast.error(e instanceof Error ? e.message : String(e));
  } finally {
    detailLoading.value = false;
  }
}

function ledgerLabel(type: string): string {
  if (type === "payment") return t("customers.payTypePayment");
  if (type === "reversal") return t("customers.payTypeReversal");
  return t("customers.payTypeCharge");
}

function ledgerBadge(type: string): string {
  if (type === "payment") return "text-bg-success";
  if (type === "reversal") return "text-bg-secondary";
  return "text-bg-danger";
}

function ledgerClass(type: string): string {
  if (type === "payment") return "text-success";
  if (type === "reversal") return "text-muted";
  return "text-danger";
}

function ledgerSign(type: string): string {
  if (type === "payment" || type === "reversal") return "−";
  return "+";
}

function openAdd() {
  editingId.value = null;
  fieldErrors.value = {};
  form.value = { name: "", phone: "", email: "", address: "", notes: "" };
  guard.capture();
  showModal.value = true;
}

function openEdit(c: Customer) {
  editingId.value = c.id;
  fieldErrors.value = {};
  form.value = {
    name: c.name,
    phone: c.phone ?? "",
    email: c.email ?? "",
    address: c.address ?? "",
    notes: c.notes ?? "",
  };
  guard.capture();
  showModal.value = true;
}

async function remove(c: Customer) {
  if (
    !(await confirmDialog({
      message: t("customers.deleteConfirm", { name: c.name }),
    }))
  )
    return;
  if (c.balance > 0.005) {
    toast.error(
      t("customers.deleteBlockedBalance", { balance: fmt(c.balance) }),
    );
    return;
  }
  try {
    const sold = await selectOne<{ n: number }>(
      "SELECT COUNT(*) AS n FROM sales WHERE customer_id = ?",
      [c.id],
    );
    if (sold && sold.n > 0) {
      toast.error(t("customers.deleteBlockedSales", { count: sold.n }));
      return;
    }
    // Ledger rows cascade automatically; nothing else can reference the row.
    await execute("DELETE FROM customers WHERE id = ?", [c.id]);
    toast.success(t("customers.deleted", { name: c.name }));
    await load();
  } catch (e: unknown) {
    toast.error(e instanceof Error ? e.message : String(e));
  }
}

function validate(): boolean {
  const errors: typeof fieldErrors.value = {};
  if (!form.value.name.trim()) {
    errors.name = t("customers.nameRequiredErr");
  }
  const email = form.value.email.trim();
  if (email && !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email)) {
    errors.email = t("customers.emailInvalid");
  }
  const phone = form.value.phone.trim();
  if (phone && !/^[+\d][\d\s\-()]{3,19}$/.test(phone)) {
    errors.phone = t("customers.phoneInvalid");
  }
  fieldErrors.value = errors;
  return Object.keys(errors).length === 0;
}

async function save() {
  if (!validate()) {
    toast.error(t("common.fixErrors"));
    return;
  }
  saving.value = true;
  try {
    if (editingId.value == null) {
      await insert(
        "INSERT INTO customers (name, phone, email, address, notes) VALUES (?, ?, ?, ?, ?)",
        [
          form.value.name.trim(),
          form.value.phone.trim() || null,
          form.value.email.trim() || null,
          form.value.address.trim() || null,
          form.value.notes.trim() || null,
        ],
      );
      toast.success(
        t("customers.customerAdded", {
          name: form.value.name.trim(),
        }),
      );
    } else {
      await execute(
        "UPDATE customers SET name = ?, phone = ?, email = ?, address = ?, notes = ? WHERE id = ?",
        [
          form.value.name.trim(),
          form.value.phone.trim() || null,
          form.value.email.trim() || null,
          form.value.address.trim() || null,
          form.value.notes.trim() || null,
          editingId.value,
        ],
      );
      toast.success(
        t("customers.customerUpdated", {
          name: form.value.name.trim(),
        }),
      );
    }
    guard.markSaved();
    showModal.value = false;
    await load();
  } catch (e) {
    fieldErrors.value = { name: e instanceof Error ? e.message : String(e) };
  } finally {
    saving.value = false;
  }
}

onMounted(async () => {
  await Promise.allSettled([load(), settings.load()]);
});
</script>
