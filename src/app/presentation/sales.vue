<template>
  <div>
    <div class="d-flex align-items-center justify-content-between mb-3">
      <h1 class="h4 mb-0">{{ t("sales.title") }}</h1>
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
            :class="{ active: filter === 'completed' }"
            @click="filter = 'completed'"
          >
            {{ t("sales.completed") }}
          </button>
          <button
            type="button"
            class="btn btn-outline-secondary"
            :class="{ active: filter === 'voided' }"
            @click="filter = 'voided'"
          >
            {{ t("sales.voided") }}
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
          :placeholder="t('sales.searchPlaceholder')"
        />
      </div>
      <div class="table-responsive">
        <table class="table table-hover align-middle mb-0">
          <thead v-if="sales.length">
            <tr>
              <th>#</th>
              <th>{{ t("sales.saleNo") }}</th>
              <th>{{ t("common.date") }}</th>
              <th>{{ t("common.cashier") }}</th>
              <th>{{ t("common.customer") }}</th>
              <th class="text-end">{{ t("sales.items") }}</th>
              <th class="text-end">{{ t("common.total") }}</th>
              <th>{{ t("common.status") }}</th>
              <th></th>
            </tr>
          </thead>
          <tbody v-if="loading">
            <tr>
              <td colspan="9" class="py-3">
                <div class="skeleton mb-2" style="height: 0.8rem"></div>
                <div
                  class="skeleton mb-2"
                  style="width: 88%; height: 0.8rem"
                ></div>
                <div
                  class="skeleton mb-2"
                  style="width: 94%; height: 0.8rem"
                ></div>
                <div class="skeleton" style="width: 90%; height: 0.8rem"></div>
              </td>
            </tr>
          </tbody>
          <tbody v-else-if="!sales.length">
            <tr>
              <td colspan="9" class="p-0 border-0">
                <EmptyState :image="emptySales" :message="t('sales.noSales')" />
              </td>
            </tr>
          </tbody>
          <tbody v-for="sale in sales" :key="sale.id">
            <tr>
              <td class="text-muted">{{ sale.id }}</td>
              <td class="fw-semibold">{{ sale.saleNo }}</td>
              <td class="text-muted">{{ dateLabel(sale.createdAt) }}</td>
              <td>{{ sale.userName || "—" }}</td>
              <td>{{ sale.customerName || "—" }}</td>
              <td class="text-end">{{ sale.itemCount }}</td>
              <td
                class="text-end fw-semibold"
                :title="
                  sale.refundedAmount > 0.005
                    ? `${fmt(sale.total)} − ${fmt(sale.refundedAmount)}`
                    : undefined
                "
              >
                {{ fmt(netTotal(sale)) }}
              </td>
              <td>
                <span
                  class="badge"
                  :class="
                    sale.status === 'voided'
                      ? 'text-bg-danger'
                      : 'text-bg-success'
                  "
                >
                  {{
                    sale.status === "voided"
                      ? t("sales.voided")
                      : t("sales.completed")
                  }}
                </span>
                <span
                  v-if="sale.refundedAmount > 0.005"
                  class="badge ms-1"
                  :class="
                    sale.total - sale.refundedAmount <= 0.005
                      ? 'text-bg-warning'
                      : 'text-bg-info'
                  "
                >
                  {{
                    sale.total - sale.refundedAmount <= 0.005
                      ? t("sales.refundedFull")
                      : t("sales.refundedPartial")
                  }}
                </span>
              </td>
              <td class="text-end">
                <button
                  v-if="sale.status === 'completed' || sale.status === 'voided'"
                  class="btn btn-sm btn-outline-secondary mx-1"
                  type="button"
                  :disabled="printingId != null"
                  :title="t('sales.printReceiptTitle')"
                  @click="reprintReceipt(sale)"
                >
                  <span
                    v-if="printingId === sale.id"
                    class="spinner-border spinner-border-sm"
                    role="status"
                  ></span>
                  <i v-else class="bi bi-printer mx-1"></i
                  >{{ t("sales.receipt") }}
                </button>
                <button
                  v-if="sale.status === 'completed'"
                  class="btn btn-sm btn-outline-warning mx-1"
                  type="button"
                  :title="t('sales.refund')"
                  @click="openRefund(sale)"
                >
                  <i class="bi bi-arrow-counterclockwise mx-1"></i
                  >{{ t("sales.refund") }}
                </button>
                <button
                  v-if="sale.status === 'completed' && openVoid"
                  class="btn btn-sm btn-outline-danger"
                  type="button"
                  @click="openVoidModal(sale)"
                >
                  <i class="bi bi-x-circle mx-1"></i>{{ t("sales.void") }}
                </button>
                <span
                  v-else-if="sale.voidReason"
                  class="text-muted small"
                  :title="sale.voidReason"
                >
                  {{ sale.voidReason }}
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

    <div
      v-if="voidTarget"
      class="modal fade show d-block"
      tabindex="-1"
      role="dialog"
      @click.self="voidTarget = null"
    >
      <div class="modal-dialog" role="document">
        <div class="modal-content">
          <div class="modal-header">
            <h5 class="modal-title">
              {{ t("sales.voidSaleTitle", { saleNo: voidTarget.saleNo }) }}
            </h5>
            <button
              type="button"
              class="btn-close"
              :aria-label="t('common.close')"
              @click="voidTarget = null"
            ></button>
          </div>
          <div class="modal-body">
            <p class="small mb-2">
              {{
                t("sales.voidBody", {
                  saleNo: voidTarget.saleNo,
                  total: fmt(voidTarget.total),
                })
              }}
            </p>
            <label class="form-label" for="void-reason">{{
              t("common.reason")
            }}</label>
            <textarea
              id="void-reason"
              v-model="voidReason"
              class="form-control"
              :class="{ 'is-invalid': !!voidError }"
              rows="3"
              :placeholder="t('sales.voidReasonPlaceholder')"
              @input="voidError = ''"
            ></textarea>
            <div v-if="voidError" class="invalid-feedback d-block">
              {{ voidError }}
            </div>
            <div class="d-flex justify-content-end gap-2 mt-3 pt-3 border-top">
              <button
                type="button"
                class="btn btn-secondary"
                :disabled="voiding"
                @click="voidTarget = null"
              >
                {{ t("common.cancel") }}
              </button>
              <AsyncButton
                variant="danger"
                :loading="voiding"
                :disabled="!canVoid"
                @click="confirmVoid"
              >
                <i v-if="!voiding" class="bi bi-x-circle mx-1"></i
                >{{ t("sales.voidConfirm") }}
              </AsyncButton>
            </div>
          </div>
        </div>
      </div>
    </div>

    <div
      v-if="refundTarget"
      class="modal fade show d-block"
      tabindex="-1"
      role="dialog"
      @click.self="refundTarget = null"
    >
      <div class="modal-dialog modal-lg modal-dialog-scrollable" role="document">
        <div class="modal-content">
          <div class="modal-header">
            <h5 class="modal-title">
              {{ t("sales.refundTitle", { saleNo: refundTarget.saleNo }) }}
            </h5>
            <button
              type="button"
              class="btn-close"
              :aria-label="t('common.close')"
              @click="refundTarget = null"
            ></button>
          </div>
          <div class="modal-body">
            <div v-if="loadingRefund" class="text-center py-4">
              <div class="spinner-border" role="status"></div>
            </div>
            <template v-else-if="refundData">
              <p class="small mb-2">
                {{ t("common.total") }}:
                <strong>{{ fmt(refundData.total) }}</strong>
                <span
                  v-if="refundData.refundedAmount > 0.005"
                  class="text-muted ms-2"
                >
                  {{
                    t("sales.alreadyRefunded", {
                      amount: fmt(refundData.refundedAmount),
                    })
                  }}
                </span>
              </p>
              <div class="table-responsive mb-2">
                <table class="table table-sm align-middle mb-0">
                  <thead>
                    <tr>
                      <th>{{ t("common.name") }}</th>
                      <th class="text-end">{{ t("products.sell") }}</th>
                      <th class="text-end">{{ t("sales.sold") }}</th>
                      <th class="text-end" style="width: 140px">
                        {{ t("sales.refundQty") }}
                      </th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr v-for="item in refundData.items" :key="item.saleItemId">
                      <td>{{ item.name }}</td>
                      <td class="text-end">
                        {{ fmt(item.price - item.discount) }}
                      </td>
                      <td class="text-end text-muted">
                        {{ item.qty }}
                        <span v-if="item.refundedQty > 0.005">
                          ({{
                            t("sales.refundedShort", { qty: item.refundedQty })
                          }})
                        </span>
                      </td>
                      <td class="text-end">
                        <input
                          v-if="remaining(item) > 0.005"
                          v-model.number="refundQtys[item.saleItemId]"
                          class="form-control form-control-sm text-end"
                          type="number"
                          min="0"
                          :max="remaining(item)"
                          step="any"
                        />
                        <span v-else class="text-muted small">{{
                          t("sales.refundedFull")
                        }}</span>
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>
              <div class="d-flex justify-content-between align-items-center">
                <button
                  class="btn btn-sm btn-outline-secondary"
                  type="button"
                  @click="fillAllRefund"
                >
                  <i class="bi bi-check-all mx-1"></i>{{ t("sales.refundAll") }}
                </button>
                <div>
                  {{ t("sales.refundEstimate") }}:
                  <strong>{{ fmt(refundEstimate) }}</strong>
                </div>
              </div>
              <div class="row g-2 mt-1">
                <div class="col-md-4">
                  <label class="form-label small" for="refund-method">{{
                    t("sales.refundMethod")
                  }}</label>
                  <select
                    id="refund-method"
                    v-model="refundMethod"
                    class="form-select form-select-sm"
                  >
                    <option value="cash">{{ t("sales.methodCash") }}</option>
                    <option value="card">{{ t("sales.methodCard") }}</option>
                    <option value="credit">
                      {{ t("sales.methodCredit") }}
                    </option>
                  </select>
                </div>
                <div class="col-md-8">
                  <label class="form-label small" for="refund-reason">
                    {{ t("common.reason") }}
                    <span class="text-muted fw-normal">{{
                      t("common.optional")
                    }}</span>
                  </label>
                  <input
                    id="refund-reason"
                    v-model="refundReason"
                    class="form-control form-control-sm"
                    type="text"
                    :placeholder="t('sales.voidReasonPlaceholder')"
                  />
                </div>
              </div>
              <div
                v-if="refundError"
                class="alert alert-danger py-2 small mt-3 mb-0"
                role="alert"
              >
                <i class="bi bi-exclamation-triangle mx-1"></i>{{ refundError }}
              </div>
            </template>
          </div>
          <div class="modal-footer">
            <button
              type="button"
              class="btn btn-outline-secondary"
              :disabled="refunding"
              @click="refundTarget = null"
            >
              {{ t("common.cancel") }}
            </button>
            <AsyncButton
              variant="warning"
              :loading="refunding"
              :disabled="!canRefund"
              @click="confirmRefund"
            >
              <i v-if="!refunding" class="bi bi-arrow-counterclockwise mx-1"></i
              >{{ t("sales.refundConfirm") }}
            </AsyncButton>
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
import { useFormGuard } from "../../composables/useFormGuard";
import { usePagedList } from "../../composables/usePagedList";
import { useToast } from "../../composables/useToast";
import { useAuth } from "../../composables/useAuth";
import { buildReceiptHtml, printReceipt } from "../../lib/receipt";
import { formatMoney } from "../../lib/currency";
import type { RefundableItem, RefundableSale, SaleReceipt, SaleRecord } from "../../types";
import emptySales from "../../assets/empty/sales.svg";

const auth = useAuth();
const toast = useToast();
const { t, locale } = useI18n();

const search = ref("");
const filter = ref("");

const {
  items: sales,
  loading,
  loadingMore,
  hasMore,
  reload,
  loadMore,
} = usePagedList<SaleRecord>(
  (limit, offset) =>
    invoke<SaleRecord[]>("list_sales", {
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

const voidTarget = ref<SaleRecord | null>(null);
const voidReason = ref("");
const voidError = ref("");
const voiding = ref(false);
const printingId = ref<number | null>(null);
const voidForm = computed(() => ({
  targetId: voidTarget.value?.id ?? null,
  reason: voidReason.value,
}));
const voidGuard = useFormGuard(voidForm);
const canVoid = computed(() => voidGuard.isDirty.value && !voiding.value);

function fmt(n: number): string {
  return formatMoney(n);
}

function dateLabel(d: string): string {
  const date = new Date(d + (d.includes("T") ? "" : "Z"));
  if (isNaN(date.getTime())) return d;
  return date.toLocaleString(locale.value);
}

function netTotal(sale: SaleRecord): number {
  return sale.total - Math.min(sale.refundedAmount, sale.total);
}

const openVoid = computed(() => auth.can("sales.void"));

/* ----- Refunds ----- */

const refundTarget = ref<SaleRecord | null>(null);
const refundData = ref<RefundableSale | null>(null);
const refundQtys = ref<Record<number, number>>({});
const refundMethod = ref<"cash" | "card" | "credit">("cash");
const refundReason = ref("");
const refundError = ref("");
const loadingRefund = ref(false);
const refunding = ref(false);

function remaining(item: RefundableItem): number {
  return Math.max(0, item.qty - item.refundedQty);
}

const canRefund = computed(
  () =>
    !refunding.value &&
    !loadingRefund.value &&
    Object.values(refundQtys.value).some((q) => q > 0),
);

// Mirrors the backend allocation: order-level discount/tax are spread
// proportionally across the invoice lines.
const refundEstimate = computed(() => {
  const data = refundData.value;
  if (!data || !data.items.length) return 0;
  const netAll = data.items.reduce(
    (sum, i) => sum + (i.price - i.discount) * i.qty,
    0,
  );
  if (netAll <= 0) return 0;
  const ratio = data.total / netAll;

  let allRemaining = true;
  let net = 0;
  for (const item of data.items) {
    const rem = remaining(item);
    const q = Math.min(Math.max(refundQtys.value[item.saleItemId] ?? 0, 0), rem);
    if (rem - q > 0.005) allRemaining = false;
    net += (item.price - item.discount) * q;
  }
  if (net <= 0) return 0;
  if (allRemaining) return Math.max(0, data.total - data.refundedAmount);
  return Math.round(net * ratio * 100) / 100;
});

async function openRefund(sale: SaleRecord) {
  refundTarget.value = sale;
  refundData.value = null;
  refundQtys.value = {};
  refundMethod.value = "cash";
  refundReason.value = "";
  refundError.value = "";
  loadingRefund.value = true;
  try {
    const data = await invoke<RefundableSale>("get_sale_for_refund", {
      saleId: sale.id,
    });
    if (data.fullyRefunded) {
      toast.info(t("sales.nothingToRefund"));
      refundTarget.value = null;
      return;
    }
    for (const item of data.items) refundQtys.value[item.saleItemId] = 0;
    refundData.value = data;
  } catch (e) {
    toast.error(String(e));
    refundTarget.value = null;
  } finally {
    loadingRefund.value = false;
  }
}

function fillAllRefund() {
  const data = refundData.value;
  if (!data) return;
  for (const item of data.items) {
    refundQtys.value[item.saleItemId] = remaining(item);
  }
}

async function confirmRefund() {
  const data = refundData.value;
  if (!data) return;
  const items = Object.entries(refundQtys.value)
    .map(([id, qty]) => ({ saleItemId: Number(id), qty }))
    .filter((x) => x.qty > 0);
  if (!items.length) return;

  refunding.value = true;
  refundError.value = "";
  try {
    const result = await invoke<{ refundNo: string; amount: number }>(
      "refund_sale",
      {
        input: {
          saleId: data.saleId,
          items,
          method: refundMethod.value,
          reason: refundReason.value.trim() || null,
          userId: auth.user?.id ?? null,
        },
      },
    );
    toast.success(
      t("sales.saleRefunded", {
        no: result.refundNo,
        amount: fmt(result.amount),
      }),
    );
    refundTarget.value = null;
    await reload();
  } catch (e) {
    refundError.value = String(e);
  } finally {
    refunding.value = false;
  }
}

function openVoidModal(sale: SaleRecord) {
  voidTarget.value = sale;
  voidReason.value = "";
  voidError.value = "";
  voidGuard.capture();
}

async function reprintReceipt(sale: SaleRecord) {
  printingId.value = sale.id;
  try {
    const receipt = await invoke<SaleReceipt>("get_sale_receipt", {
      input: { saleId: sale.id },
    });
    await printReceipt(buildReceiptHtml(receipt));
  } catch (e) {
    toast.error(t("sales.printingFailed", { error: String(e) }));
  } finally {
    printingId.value = null;
  }
}

async function confirmVoid() {
  if (!voidTarget.value) return;
  if (!voidReason.value.trim()) {
    voidError.value = t("sales.reasonRequired");
    toast.error(t("common.fixErrors"));
    return;
  }
  voiding.value = true;
  voidError.value = "";
  try {
    await invoke("void_sale", {
      input: {
        saleId: voidTarget.value.id,
        reason: voidReason.value.trim(),
        userId: auth.user?.id ?? null,
      },
    });
    toast.success(t("sales.saleVoided", { saleNo: voidTarget.value.saleNo }));
    voidGuard.markSaved();
    voidTarget.value = null;
    await reload();
  } catch (e) {
    voidError.value = String(e);
  } finally {
    voiding.value = false;
  }
}

onMounted(reload);
</script>
