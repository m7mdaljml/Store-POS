<template>
  <div>
    <div class="d-flex align-items-center justify-content-between mb-3">
      <h1 class="h4 mb-0">Stock Movements</h1>
      <button class="btn btn-primary" type="button" @click="openAdjust">
        <i class="bi bi-plus-lg me-1"></i>New Adjustment
      </button>
    </div>

    <div v-if="error" class="alert alert-danger py-2 small" role="alert">
      <i class="bi bi-exclamation-triangle me-1"></i>{{ error }}
    </div>
    <div v-if="notice" class="alert alert-success py-2 small" role="alert">
      <i class="bi bi-check-circle me-1"></i>{{ notice }}
    </div>

    <div class="card">
      <div class="p-2 border-bottom d-flex gap-2">
        <select v-model="typeFilter" class="form-select form-select-sm" style="width: auto">
          <option value="all">All types</option>
          <option v-for="(label, key) in TYPE_LABELS" :key="key" :value="key">{{ label }}</option>
        </select>
        <select
          v-model="productFilter"
          class="form-select form-select-sm"
          style="width: auto; max-width: 260px"
        >
          <option :value="null">All products</option>
          <option v-for="p in catalog.products" :key="p.id" :value="p.id">{{ p.name }}</option>
        </select>
      </div>
      <div class="table-responsive">
        <table class="table align-middle mb-0">
          <thead>
            <tr>
              <th>Date</th>
              <th>Product</th>
              <th>Type</th>
              <th class="text-end">Qty</th>
              <th>Notes</th>
              <th>By</th>
            </tr>
          </thead>
          <tbody>
            <tr v-if="loading">
              <td colspan="6" class="text-center text-muted py-4">Loading…</td>
            </tr>
            <tr v-else-if="!filtered.length">
              <td colspan="6" class="text-center text-muted py-4">
                No stock movements yet — use "New Adjustment" to adjust stock
              </td>
            </tr>
            <tr v-for="m in filtered" :key="m.id">
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
    </div>

    <div v-if="showModal" class="modal-backdrop show"></div>
    <div v-if="showModal" class="modal d-block" tabindex="-1">
      <div class="modal-dialog modal-dialog-centered">
        <div class="modal-content">
          <form @submit.prevent="saveAdjust">
            <div class="modal-header">
              <h5 class="modal-title">Adjust Stock</h5>
              <button type="button" class="btn-close" @click="showModal = false"></button>
            </div>
            <div class="modal-body">
              <div v-if="adjustError" class="alert alert-danger py-2 small" role="alert">
                <i class="bi bi-exclamation-triangle me-1"></i>{{ adjustError }}
              </div>
              <div class="mb-3">
                <label class="form-label" for="a-product">Product</label>
                <select
                  id="a-product"
                  v-model="selectedProductId"
                  class="form-select"
                >
                  <option v-for="p in catalog.products" :key="p.id" :value="p.id">
                    {{ p.name }} ({{ p.stock_qty }} in stock)
                  </option>
                </select>
              </div>
              <div class="row g-3">
                <div class="col-6">
                  <label class="form-label">Current stock</label>
                  <input class="form-control" type="text" :value="currentStock + ' ' + (selectedProduct?.unit ?? '')" disabled />
                </div>
                <div class="col-6">
                  <label class="form-label" for="a-new">New total</label>
                  <input
                    id="a-new"
                    v-model.number="adjustNew"
                    class="form-control"
                    type="number"
                    min="0"
                    step="any"
                    required
                  />
                </div>
              </div>
              <div class="mt-3">
                <label class="form-label" for="a-notes">Reason (optional)</label>
                <input
                  id="a-notes"
                  v-model="adjustNotes"
                  class="form-control"
                  type="text"
                  placeholder="e.g. Damaged stock, inventory count…"
                />
                <div v-if="delta !== 0" class="form-text">
                  Change:
                  <span :class="delta > 0 ? 'text-success' : 'text-danger'">
                    {{ delta > 0 ? "+" : "" }}{{ delta }}
                  </span>
                </div>
              </div>
            </div>
            <div class="modal-footer">
              <button type="button" class="btn btn-secondary" @click="showModal = false">
                Cancel
              </button>
              <button type="submit" class="btn btn-primary" :disabled="saving">
                <span
                  v-if="saving"
                  class="spinner-border spinner-border-sm me-1"
                  role="status"
                  aria-hidden="true"
                ></span>
                Save Adjustment
              </button>
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
import { useCatalogStore } from "../../stores/catalog";
import { useAuth } from "../../composables/useAuth";

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

const movements = ref<StockMovement[]>([]);
const loading = ref(false);
const error = ref("");
const notice = ref("");

const typeFilter = ref<"all" | string>("all");
const productFilter = ref<number | null>(null);

const TYPE_LABELS: Record<string, string> = {
  opening: "Opening",
  adjustment: "Adjustment",
  purchase_in: "Purchase",
  sale_out: "Sale",
  void: "Void",
};

const TYPE_BADGES: Record<string, string> = {
  opening: "text-bg-secondary",
  adjustment: "text-bg-info",
  purchase_in: "text-bg-success",
  sale_out: "text-bg-primary",
  void: "text-bg-danger",
};

const typeLabel = (t: string) => TYPE_LABELS[t] ?? t;
const typeBadge = (t: string) => TYPE_BADGES[t] ?? "text-bg-secondary";

const filtered = computed(() =>
  movements.value.filter((m) => {
    if (typeFilter.value !== "all" && m.movementType !== typeFilter.value) return false;
    if (productFilter.value != null && m.productId !== productFilter.value) return false;
    return true;
  })
);

async function load() {
  loading.value = true;
  error.value = "";
  try {
    movements.value = await invoke<StockMovement[]>("list_stock_movements");
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

const showModal = ref(false);
const saving = ref(false);
const adjustError = ref("");
const selectedProductId = ref<number | null>(null);
const adjustNew = ref(0);
const adjustNotes = ref("");

const selectedProduct = computed(() =>
  catalog.products.find((p) => p.id === selectedProductId.value) ?? null
);
const currentStock = computed(() => selectedProduct.value?.stock_qty ?? 0);
const delta = computed(() => {
  const d = adjustNew.value - currentStock.value;
  return Number.isNaN(d) ? 0 : d;
});

function openAdjust() {
  adjustError.value = "";
  adjustNew.value = 0;
  adjustNotes.value = "";
  if (selectedProductId.value == null || !catalog.products.some((p) => p.id === selectedProductId.value)) {
    selectedProductId.value = catalog.products[0]?.id ?? null;
  }
  adjustNew.value = currentStock.value;
  showModal.value = true;
}

async function saveAdjust() {
  adjustError.value = "";
  const product = selectedProduct.value;
  if (!product) {
    adjustError.value = "Select a product";
    return;
  }
  if (typeof adjustNew.value !== "number" || isNaN(adjustNew.value) || adjustNew.value < 0) {
    adjustError.value = "Enter a valid new stock total (0 or more)";
    return;
  }
  if (delta.value === 0) {
    adjustError.value = "New total is the same as current stock — nothing to change";
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
    showModal.value = false;
    notice.value = `Stock for "${product.name}" adjusted`;
    await Promise.all([load(), catalog.load()]);
  } catch (e) {
    adjustError.value = e instanceof Error ? e.message : String(e);
  } finally {
    saving.value = false;
  }
}

watch(
  selectedProductId,
  (id, oldId) => {
    if (showModal.value && id !== oldId) {
      adjustNew.value = currentStock.value;
    }
  }
);

onMounted(async () => {
  await Promise.allSettled([load(), catalog.load()]);
});
</script>
