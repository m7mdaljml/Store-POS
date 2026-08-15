<template>
  <div class="page-container">
    <div class="page-heading d-flex flex-wrap align-items-center gap-3">
      <div>
        <h2 class="mb-0">{{ t("sales.title") }}</h2>
        <span class="text-muted small">{{ t("sales.subtitle") }}</span>
      </div>
      <div class="ms-auto d-flex align-items-center gap-2">
        <div class="btn-group btn-group-sm" role="group" :aria-label="t('sales.filterStatusAria')">
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
        <button class="btn btn-sm btn-outline-primary" type="button" @click="load">
          <i class="bi bi-arrow-clockwise me-1"></i>{{ t("common.refresh") }}
        </button>
      </div>
    </div>

    <div v-if="error" class="alert alert-warning py-1 px-2 small" role="alert">
      <i class="bi bi-exclamation-triangle me-1"></i>{{ error }}
    </div>
    <div v-if="notice" class="alert alert-success py-1 px-2 small" role="alert">
      <i class="bi bi-check-circle me-1"></i>{{ notice }}
    </div>

    <div class="card">
      <div class="table-responsive">
        <table class="table table-hover align-middle mb-0">
          <thead>
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
          <tbody>
            <tr v-if="loading">
              <td colspan="9" class="text-center py-4 text-muted">
                <span class="spinner-border spinner-border-sm me-2" role="status"></span>{{ t("common.loading") }}
              </td>
            </tr>
            <tr v-else-if="!filteredSales.length">
              <td colspan="9" class="text-center py-4 text-muted">{{ t("sales.noSales") }}</td>
            </tr>
            <tr v-for="sale in filteredSales" :key="sale.id">
              <td class="text-muted">{{ sale.id }}</td>
              <td class="fw-semibold">{{ sale.saleNo }}</td>
              <td class="text-muted">{{ dateLabel(sale.createdAt) }}</td>
              <td>{{ sale.userName || "—" }}</td>
              <td>{{ sale.customerName || "—" }}</td>
              <td class="text-end">{{ sale.itemCount }}</td>
              <td class="text-end fw-semibold">{{ fmt(sale.total) }}</td>
              <td>
                <span
                  class="badge"
                  :class="sale.status === 'voided' ? 'text-bg-danger' : 'text-bg-success'"
                >
                  {{ sale.status === "voided" ? t("sales.voided") : t("sales.completed") }}
                </span>
              </td>
              <td class="text-end">
                <button
                  v-if="sale.status === 'completed' || sale.status === 'voided'"
                  class="btn btn-sm btn-outline-secondary me-1"
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
                  <i v-else class="bi bi-printer me-1"></i>{{ t("sales.receipt") }}
                </button>
                <button
                  v-if="sale.status === 'completed' && openVoid"
                  class="btn btn-sm btn-outline-danger"
                  type="button"
                  @click="openVoidModal(sale)"
                >
                  <i class="bi bi-x-circle me-1"></i>{{ t("sales.void") }}
                </button>
                <span v-else-if="sale.voidReason" class="text-muted small" :title="sale.voidReason">
                  {{ sale.voidReason }}
                </span>
              </td>
            </tr>
          </tbody>
        </table>
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
            <h5 class="modal-title">{{ t("sales.voidSaleTitle", { saleNo: voidTarget.saleNo }) }}</h5>
            <button
              type="button"
              class="btn-close"
              :aria-label="t('common.close')"
              @click="voidTarget = null"
            ></button>
          </div>
          <div class="modal-body">
            <p class="small mb-2">
              {{ t("sales.voidBody", { saleNo: voidTarget.saleNo, total: fmt(voidTarget.total) }) }}
            </p>
            <label class="form-label" for="void-reason">{{ t("common.reason") }}</label>
            <textarea
              id="void-reason"
              v-model="voidReason"
              class="form-control"
              rows="3"
              :placeholder="t('sales.voidReasonPlaceholder')"
            ></textarea>
            <div v-if="voidError" class="alert alert-warning py-1 px-2 mt-2 small" role="alert">
              {{ voidError }}
            </div>
          </div>
          <div class="modal-footer">
            <button
              type="button"
              class="btn btn-secondary"
              :disabled="voiding"
              @click="voidTarget = null"
            >
              {{ t("common.cancel") }}
            </button>
            <button
              type="button"
              class="btn btn-danger"
              :disabled="voiding"
              @click="confirmVoid"
            >
              <span v-if="voiding" class="spinner-border spinner-border-sm me-2" role="status"></span>
              <i v-else class="bi bi-x-circle me-1"></i>{{ t("sales.voidConfirm") }}
            </button>
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
import { useSettingsStore } from "../../stores/settings";
import { useAuth } from "../../composables/useAuth";
import { buildReceiptHtml, printReceipt } from "../../lib/receipt";
import type { SaleReceipt, SaleRecord } from "../../types";

const settings = useSettingsStore();
const auth = useAuth();
const { t, locale } = useI18n();

const sales = ref<SaleRecord[]>([]);
const loading = ref(false);
const error = ref("");
const notice = ref("");
const filter = ref("");

const voidTarget = ref<SaleRecord | null>(null);
const voidReason = ref("");
const voidError = ref("");
const voiding = ref(false);
const printingId = ref<number | null>(null);

function fmt(n: number): string {
  if (!settings.currency) return n.toFixed(2);
  return new Intl.NumberFormat(locale.value, {
    style: "currency",
    currency: settings.currency,
    currencyDisplay: "narrowSymbol",
  }).format(n);
}

function dateLabel(d: string): string {
  const date = new Date(d + (d.includes("T") ? "" : "Z"));
  if (isNaN(date.getTime())) return d;
  return date.toLocaleString(locale.value);
}

const filteredSales = computed(() =>
  filter.value ? sales.value.filter((s) => s.status === filter.value) : sales.value
);

const openVoid = computed(() => auth.can("sales.void"));

async function load() {
  loading.value = true;
  error.value = "";
  try {
    sales.value = await invoke<SaleRecord[]>("list_sales", { input: { limit: 200 } });
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

function openVoidModal(sale: SaleRecord) {
  voidTarget.value = sale;
  voidReason.value = "";
  voidError.value = "";
}

async function reprintReceipt(sale: SaleRecord) {
  printingId.value = sale.id;
  error.value = "";
  try {
    const receipt = await invoke<SaleReceipt>("get_sale_receipt", {
      input: { saleId: sale.id },
    });
    await printReceipt(buildReceiptHtml(receipt));
  } catch (e) {
    error.value = t("sales.printingFailed", { error: String(e) });
  } finally {
    printingId.value = null;
  }
}

async function confirmVoid() {
  if (!voidTarget.value) return;
  if (!voidReason.value.trim()) {
    voidError.value = t("sales.reasonRequired");
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
    notice.value = t("sales.saleVoided", { saleNo: voidTarget.value.saleNo });
    voidTarget.value = null;
    await load();
  } catch (e) {
    voidError.value = String(e);
  } finally {
    voiding.value = false;
  }
}

onMounted(load);
</script>
