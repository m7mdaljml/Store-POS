<template>
  <div>
    <div class="d-flex align-items-center justify-content-between mb-3">
      <h1 class="h4 mb-0">{{ t("stock.title") }}</h1>
      <button class="btn btn-primary" type="button" @click="openAdjust">
        <i class="bi bi-plus-lg mx-1"></i>{{ t("stock.newAdjustment") }}
      </button>
    </div>

    <div class="card">
      <div class="p-2 border-bottom d-flex gap-2">
        <select
          v-model="typeFilter"
          class="form-select form-select-sm"
          style="width: auto"
        >
          <option value="all">{{ t("stock.allTypes") }}</option>
          <option v-for="key in TYPE_ORDER" :key="key" :value="key">
            {{ t("stock.typeLabels." + key) }}
          </option>
        </select>
        <AppSelect
          v-model="productFilter"
          sm
          class="app-select-inline"
          :items="catalog.products"
          :option-label="(p) => p.name"
          :option-value="(p) => p.id"
          :placeholder="t('stock.allProducts')"
        />
        <input
          v-model="search"
          class="form-control form-control-sm flex-grow-1"
          type="search"
          :placeholder="t('stock.searchPlaceholder')"
        />
      </div>
      <div class="table-responsive">
        <table class="table align-middle mb-0">
          <thead v-if="movements.length">
            <tr>
              <th>{{ t("common.date") }}</th>
              <th>{{ t("common.product") }}</th>
              <th>{{ t("stock.type") }}</th>
              <th class="text-end">{{ t("stock.qty") }}</th>
              <th>{{ t("common.notes") }}</th>
              <th>{{ t("stock.by") }}</th>
            </tr>
          </thead>
          <tbody v-if="loading">
            <tr>
              <td colspan="6" class="text-center text-muted py-4">
                {{ t("common.loading") }}
              </td>
            </tr>
          </tbody>
          <tbody v-else-if="!movements.length">
            <tr>
              <td colspan="6" class="p-0 border-0">
                <EmptyState
                  :image="emptyStock"
                  :message="t('stock.noMovements')"
                />
              </td>
            </tr>
          </tbody>
          <tbody v-for="m in movements" :key="m.id">
            <tr>
              <td class="text-muted small text-nowrap">{{ m.createdAt }}</td>
              <td class="fw-semibold">{{ m.productName }}</td>
              <td>
                <span class="badge" :class="typeBadge(m.movementType)">
                  {{ typeLabel(m.movementType) }}
                </span>
              </td>
              <td
                class="text-end fw-semibold"
                :class="m.qty >= 0 ? 'text-success' : 'text-danger'"
              >
                {{ m.qty >= 0 ? "+" : "" }}{{ m.qty }}
              </td>
              <td class="text-muted small">{{ m.notes ?? "—" }}</td>
              <td class="text-muted small">{{ m.userName ?? "—" }}</td>
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
          <form @submit.prevent="saveAdjust">
            <div class="modal-header">
              <h5 class="modal-title">{{ t("stock.adjustTitle") }}</h5>
              <button
                type="button"
                class="btn-close"
                @click="showModal = false"
              ></button>
            </div>
            <div class="modal-body">
              <div
                v-if="adjustError"
                class="alert alert-danger py-2 small"
                role="alert"
              >
                <i class="bi bi-exclamation-triangle mx-1"></i>{{ adjustError }}
              </div>
              <div class="mb-3">
                <label class="form-label" for="a-product">{{
                  t("common.product")
                }}</label>
                <AppSelect
                  id="a-product"
                  v-model="selectedProductId"
                  :items="catalog.products"
                  :option-label="
                    (p) => `${p.name} (${p.stock_qty} ${t('stock.inStock')})`
                  "
                  :option-value="(p) => p.id"
                />
              </div>
              <div class="row g-3">
                <div class="col-6">
                  <label class="form-label">{{
                    t("products.currentStock")
                  }}</label>
                  <input
                    class="form-control"
                    type="text"
                    :value="currentStock + ' ' + (selectedProduct?.unit ?? '')"
                    disabled
                  />
                </div>
                <div class="col-6">
                  <label class="form-label" for="a-new">{{
                    t("products.newStockTotal")
                  }}</label>
                  <input
                    id="a-new"
                    v-model.number="adjustNew"
                    class="form-control"
                    type="number"
                    min="0"
                    step="1"
                    required
                  />
                </div>
              </div>
              <div class="mt-3">
                <label class="form-label" for="a-notes">{{
                  t("products.reasonOptional")
                }}</label>
                <input
                  id="a-notes"
                  v-model="adjustNotes"
                  class="form-control"
                  type="text"
                  :placeholder="t('stock.reasonPlaceholder')"
                />
                <div v-if="delta !== 0" class="form-text">
                  {{ t("stock.change") }}:
                  <span :class="delta > 0 ? 'text-success' : 'text-danger'">
                    {{ delta > 0 ? "+" : "" }}{{ delta }}
                  </span>
                </div>
              </div>
              <div
                class="d-flex justify-content-end gap-2 mt-3 pt-3 border-top"
              >
                <button
                  type="button"
                  class="btn btn-secondary"
                  @click="showModal = false"
                >
                  {{ t("common.cancel") }}
                </button>
                <AsyncButton
                  type="submit"
                  :loading="saving"
                  :disabled="!canAdjust"
                >
                  {{ t("stock.saveAdjustment") }}
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
import { computed, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useI18n } from "vue-i18n";
import EmptyState from "../../components/EmptyState.vue";
import AsyncButton from "../../components/AsyncButton.vue";
import Paginator from "../../components/Paginator.vue";
import AppSelect from "../../components/AppSelect.vue";
import { useFormGuard } from "../../composables/useFormGuard";
import { usePagedList, type Paged } from "../../composables/usePagedList";
import { useToast } from "../../composables/useToast";
import { useCatalogStore } from "../../stores/catalog";
import { useAuth } from "../../composables/useAuth";
import emptyStock from "../../assets/empty/stock.svg";

interface StockMovement {
  id: number;
  productId: number;
  productName: string;
  movementType: string;
  qty: number;
  notes: string | null;
  userName: string | null;
  createdAt: string;
}

const catalog = useCatalogStore();
const auth = useAuth();
const toast = useToast();
const { t } = useI18n();

const search = ref("");
const typeFilter = ref<"all" | string>("all");
const productFilter = ref<number | null>(null);

const {
  items: movements,
  loading,
  page,
  size,
  totalItems,
  goToPage,
  reload,
} = usePagedList<StockMovement>(
  (limit, offset) =>
    invoke<Paged<StockMovement>>("list_stock_movements", {
      movementType: typeFilter.value === "all" ? null : typeFilter.value,
      productId: productFilter.value,
      search: search.value.trim() || null,
      limit,
      offset,
    }),
  [typeFilter, productFilter, search],
  (e) => toast.error(e instanceof Error ? e.message : String(e)),
);

const TYPE_ORDER = [
  "opening",
  "adjustment",
  "purchase_in",
  "sale_out",
  "sale_refund_in",
  "void",
];

const TYPE_BADGES: Record<string, string> = {
  opening: "text-bg-secondary",
  adjustment: "text-bg-info",
  purchase_in: "text-bg-success",
  sale_out: "text-bg-primary",
  sale_refund_in: "text-bg-warning",
  void: "text-bg-danger",
};

const typeLabel = (key: string) =>
  TYPE_ORDER.includes(key) ? t("stock.typeLabels." + key) : key;
const typeBadge = (key: string) => TYPE_BADGES[key] ?? "text-bg-secondary";

const showModal = ref(false);
const saving = ref(false);
const adjustError = ref("");
const selectedProductId = ref<number | null>(null);
const adjustNew = ref<number | null>(0);
const adjustNotes = ref("");

const adjustForm = computed(() => ({
  productId: selectedProductId.value,
  newQty: adjustNew.value,
  notes: adjustNotes.value,
}));
const adjustGuard = useFormGuard(adjustForm);
const canAdjust = computed(() => adjustGuard.isDirty.value && !saving.value);

const selectedProduct = computed(
  () => catalog.products.find((p) => p.id === selectedProductId.value) ?? null,
);
const currentStock = computed(() => selectedProduct.value?.stock_qty ?? 0);
const delta = computed(() => {
  if (typeof adjustNew.value !== "number") return 0;
  const d = adjustNew.value - currentStock.value;
  return Number.isNaN(d) ? 0 : d;
});

function openAdjust() {
  adjustError.value = "";
  adjustNew.value = 0;
  adjustNotes.value = "";
  if (
    selectedProductId.value == null ||
    !catalog.products.some((p) => p.id === selectedProductId.value)
  ) {
    selectedProductId.value = catalog.products[0]?.id ?? null;
  }
  adjustNew.value = currentStock.value;
  showModal.value = true;
  adjustGuard.capture();
}

async function saveAdjust() {
  adjustError.value = "";
  const product = selectedProduct.value;
  const newQty = adjustNew.value;
  if (!product) {
    adjustError.value = t("stock.selectProduct");
    toast.error(t("common.fixErrors"));
    return;
  }
  if (typeof newQty !== "number" || isNaN(newQty) || newQty < 0) {
    adjustError.value = t("stock.invalidNewTotal");
    toast.error(t("common.fixErrors"));
    return;
  }
  if (delta.value === 0) {
    adjustError.value = t("stock.noChange");
    return;
  }
  saving.value = true;
  try {
    await invoke("adjust_stock", {
      productId: product.id,
      qty: delta.value,
      notes: adjustNotes.value.trim() || null,
      userId: auth.user?.id ?? null,
    });
    adjustGuard.markSaved();
    showModal.value = false;
    toast.success(t("stock.adjusted", { name: product.name }));
    await Promise.all([reload(), catalog.load()]);
  } catch (e) {
    adjustError.value = e instanceof Error ? e.message : String(e);
  } finally {
    saving.value = false;
  }
}

watch(selectedProductId, (id, oldId) => {
  if (showModal.value && id !== oldId) {
    adjustNew.value = currentStock.value;
  }
});

onMounted(async () => {
  await Promise.allSettled([reload(), catalog.load()]);
});
</script>

<style scoped>
.app-select-inline {
  width: auto;
  min-width: 220px;
  max-width: 280px;
}
</style>
