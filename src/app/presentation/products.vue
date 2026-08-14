<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useCatalogStore } from "../../stores/catalog";
import { useSettingsStore } from "../../stores/settings";
import { select } from "../../lib/db";
import type { Category, Product, TaxProfile } from "../../types";

const catalog = useCatalogStore();
const settings = useSettingsStore();

const categories = ref<Category[]>([]);
const selected = ref<number | null>(null);
const search = ref("");
const loading = ref(false);
const error = ref("");
const notice = ref("");

const showModal = ref(false);
const saving = ref(false);
const productError = ref("");
const editingId = ref<number | null>(null);
const form = ref({
  name: "",
  sku: "",
  barcode: "",
  description: "",
  categoryId: null as number | null,
  costPrice: 0,
  sellPrice: 0,
  taxProfileId: null as number | null,
  unit: "pc",
  reorderLevel: 0,
  isActive: true,
});

const showCatModal = ref(false);
const catSaving = ref(false);
const categoryError = ref("");
const catEditingId = ref<number | null>(null);
const catForm = ref({ name: "", parentId: null as number | null });

const adjustTarget = ref<Product | null>(null);
const adjustNew = ref(0);
const adjustNotes = ref("");
const adjustSaving = ref(false);
const adjustError = ref("");

const taxProfiles = ref<TaxProfile[]>([]);

function fmt(n: number): string {
  if (!settings.currency) return n.toFixed(2);
  return new Intl.NumberFormat(undefined, {
    style: "currency",
    currency: settings.currency,
    currencyDisplay: "narrowSymbol",
  }).format(n);
}

const filteredProducts = computed(() => {
  const q = search.value.trim().toLowerCase();
  return catalog.products.filter((p) => {
    if (selected.value != null && p.category_id !== selected.value) return false;
    if (!q) return true;
    return (
      p.name.toLowerCase().includes(q) ||
      (p.sku?.toLowerCase().includes(q) ?? false) ||
      (p.barcode?.toLowerCase().includes(q) ?? false)
    );
  });
});

const categoryName = (id: number | null) =>
  categories.value.find((c) => c.id === id)?.name ?? "—";

async function loadCategories() {
  loading.value = true;
  error.value = "";
  try {
    categories.value = await invoke<Category[]>("list_categories");
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

/* ----- Categories ----- */

function openAddCategory() {
  categoryError.value = "";
  catEditingId.value = null;
  catForm.value = { name: "", parentId: null };
  showCatModal.value = true;
}

function openEditCategory(c: Category) {
  categoryError.value = "";
  catEditingId.value = c.id;
  catForm.value = { name: c.name, parentId: c.parentId ?? null };
  showCatModal.value = true;
}

async function saveCategory() {
  categoryError.value = "";
  if (!catForm.value.name.trim()) {
    categoryError.value = "Category name is required";
    return;
  }
  catSaving.value = true;
  try {
    if (catEditingId.value == null) {
      await invoke<number>("create_category", {
        name: catForm.value.name,
        parentId: catForm.value.parentId,
      });
      notice.value = "Category added";
    } else {
      await invoke("update_category", {
        categoryId: catEditingId.value,
        name: catForm.value.name,
        parentId: catForm.value.parentId,
      });
      notice.value = "Category updated";
    }
    showCatModal.value = false;
    await Promise.all([loadCategories(), catalog.load()]);
  } catch (e) {
    categoryError.value = e instanceof Error ? e.message : String(e);
  } finally {
    catSaving.value = false;
  }
}

async function removeCategory(c: Category) {
  error.value = "";
  if (!window.confirm(`Delete category "${c.name}"?`)) return;
  try {
    await invoke("delete_category", { categoryId: c.id });
    notice.value = `"${c.name}" deleted`;
    if (selected.value === c.id) selected.value = null;
    await Promise.all([loadCategories(), catalog.load()]);
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  }
}

/* ----- Products ----- */

function openAddProduct() {
  productError.value = "";
  editingId.value = null;
  form.value = {
    name: "",
    sku: "",
    barcode: "",
    description: "",
    categoryId: selected.value,
    costPrice: 0,
    sellPrice: 0,
    taxProfileId: null,
    unit: "pc",
    reorderLevel: 0,
    isActive: true,
  };
  showModal.value = true;
}

function openEditProduct(p: Product) {
  productError.value = "";
  editingId.value = p.id;
  form.value = {
    name: p.name,
    sku: p.sku ?? "",
    barcode: p.barcode ?? "",
    description: p.description ?? "",
    categoryId: p.category_id,
    costPrice: p.cost_price,
    sellPrice: p.sell_price,
    taxProfileId: p.tax_profile_id,
    unit: p.unit,
    reorderLevel: p.reorder_level,
    isActive: p.is_active === 1,
  };
  showModal.value = true;
}

function productPayload() {
  return {
    name: form.value.name,
    sku: form.value.sku || null,
    barcode: form.value.barcode || null,
    description: form.value.description || null,
    categoryId: form.value.categoryId,
    costPrice: form.value.costPrice,
    sellPrice: form.value.sellPrice,
    taxProfileId: form.value.taxProfileId,
    unit: form.value.unit,
    reorderLevel: form.value.reorderLevel,
    imagePath: null,
    isActive: form.value.isActive,
  };
}

async function saveProduct() {
  productError.value = "";
  if (!form.value.name.trim()) {
    productError.value = "Product name is required";
    return;
  }
  saving.value = true;
  try {
    if (editingId.value == null) {
      await invoke<number>("create_product", { input: productPayload() });
      notice.value = "Product added";
    } else {
      await invoke("update_product", { productId: editingId.value, input: productPayload() });
      notice.value = "Product updated";
    }
    showModal.value = false;
    await Promise.all([loadCategories(), catalog.load()]);
  } catch (e) {
    productError.value = e instanceof Error ? e.message : String(e);
  } finally {
    saving.value = false;
  }
}

async function removeProduct(p: Product) {
  productError.value = "";
  if (!window.confirm(`Delete "${p.name}" permanently? This cannot be undone.`)) return;
  try {
    await invoke("delete_product", { productId: p.id });
    notice.value = `"${p.name}" deleted`;
    await Promise.all([loadCategories(), catalog.load()]);
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  }
}

function openAdjust(p: Product) {
  adjustError.value = "";
  adjustTarget.value = p;
  adjustNew.value = p.stock_qty;
  adjustNotes.value = "";
}

async function saveAdjust() {
  const target = adjustTarget.value;
  if (!target) return;
  adjustError.value = "";
  const delta = adjustNew.value - target.stock_qty;
  if (delta === 0) {
    adjustTarget.value = null;
    return;
  }
  adjustSaving.value = true;
  try {
    await invoke("adjust_stock", {
      productId: target.id,
      qty: delta,
      notes: adjustNotes.value || null,
    });
    adjustTarget.value = null;
    notice.value = `Stock updated: ${target.stock_qty} → ${adjustNew.value} ${target.unit}`;
    await Promise.all([loadCategories(), catalog.load()]);
  } catch (e) {
    adjustError.value = e instanceof Error ? e.message : String(e);
  } finally {
    adjustSaving.value = false;
  }
}

onMounted(async () => {
  taxProfiles.value = await select<TaxProfile>("SELECT id, name, rate FROM tax_profiles ORDER BY name").catch(
    () => []
  );
  await Promise.allSettled([loadCategories(), catalog.load(), settings.load()]);
});
</script>

<template>
  <div>
    <div class="d-flex align-items-center justify-content-between mb-3">
      <h1 class="h4 mb-0">Products</h1>
      <button class="btn btn-primary" type="button" @click="openAddProduct">
        <i class="bi bi-plus-lg me-1"></i>Add Product
      </button>
    </div>

    <div v-if="error" class="alert alert-danger py-2 small" role="alert">
      <i class="bi bi-exclamation-triangle me-1"></i>{{ error }}
    </div>
    <div v-if="notice" class="alert alert-success py-2 small" role="alert">
      <i class="bi bi-check-circle me-1"></i>{{ notice }}
    </div>

    <div class="d-flex gap-3 align-items-start">
      <div class="card category-sidebar">
        <button class="btn btn-sm btn-primary w-100" type="button" @click="openAddCategory">
          <i class="bi bi-plus-lg me-1"></i>Add Category
        </button>
        <div class="mt-2">
          <button
            class="cat-item"
            type="button"
            :class="{ active: selected == null }"
            @click="selected = null"
          >
            <i class="bi bi-box-seam"></i>
            <span>All Products</span>
            <span class="badge text-bg-secondary">{{ catalog.products.length }}</span>
          </button>
          <button
            v-for="c in categories"
            :key="c.id"
            class="cat-item"
            type="button"
            :class="{ active: selected === c.id }"
            @click="selected = c.id"
          >
            <i class="bi bi-folder2"></i>
            <span>{{ c.name }}</span>
            <span class="badge text-bg-secondary">{{ c.productCount }}</span>
            <span class="cat-actions" @click.stop>
              <button
                class="btn btn-sm btn-link p-0 me-1"
                type="button"
                title="Edit"
                @click="openEditCategory(c)"
              >
                <i class="bi bi-pencil"></i>
              </button>
              <button
                class="btn btn-sm btn-link p-0 text-danger"
                type="button"
                title="Delete"
                @click="removeCategory(c)"
              >
                <i class="bi bi-trash"></i>
              </button>
            </span>
          </button>
          <div v-if="!categories.length && !loading" class="text-muted small p-2">
            No categories yet
          </div>
        </div>
      </div>

      <div class="card flex-grow-1">
        <div class="p-2 border-bottom">
          <input
            v-model="search"
            class="form-control form-control-sm"
            type="search"
            placeholder="Search by product name, SKU or barcode…"
          />
        </div>
        <div class="table-responsive">
          <table class="table align-middle mb-0">
            <thead>
              <tr>
                <th>Name</th>
                <th>Category</th>
                <th>SKU / Barcode</th>
                <th class="text-end">Cost</th>
                <th class="text-end">Sell</th>
                <th class="text-end">Stock</th>
                <th>Status</th>
                <th class="text-end">Actions</th>
              </tr>
            </thead>
            <tbody>
              <tr v-if="!filteredProducts.length">
                <td colspan="8" class="text-center text-muted py-4">
                  No products yet — click "Add Product" to create one
                </td>
              </tr>
              <tr v-for="p in filteredProducts" :key="p.id">
                <td class="fw-semibold">{{ p.name }}</td>
                <td class="text-muted">{{ categoryName(p.category_id) }}</td>
                <td class="text-muted small">
                  <div v-if="p.sku">SKU: {{ p.sku }}</div>
                  <div v-if="p.barcode">{{ p.barcode }}</div>
                  <span v-if="!p.sku && !p.barcode">—</span>
                </td>
                <td class="text-end">{{ fmt(p.cost_price) }}</td>
                <td class="text-end">{{ fmt(p.sell_price) }}</td>
                <td class="text-end">{{ p.stock_qty }} {{ p.unit }}</td>
                <td>
                  <span
                    class="badge"
                    :class="p.is_active ? 'text-bg-success' : 'text-bg-secondary'"
                  >
                    {{ p.is_active ? "Active" : "Inactive" }}
                  </span>
                </td>
                <td class="text-end text-nowrap">
                  <button
                    class="btn btn-sm btn-outline-secondary me-1"
                    type="button"
                    title="Adjust stock"
                    @click="openAdjust(p)"
                  >
                    <i class="bi bi-box-arrow-up-down me-1"></i>Stock
                  </button>
                  <button
                    class="btn btn-sm btn-outline-primary me-1"
                    type="button"
                    title="Edit"
                    @click="openEditProduct(p)"
                  >
                    <i class="bi bi-pencil-square"></i>
                  </button>
                  <button
                    class="btn btn-sm btn-outline-danger"
                    type="button"
                    title="Delete"
                    @click="removeProduct(p)"
                  >
                    <i class="bi bi-trash"></i>
                  </button>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>

    <div v-if="showModal" class="modal-backdrop show"></div>
    <div v-if="showModal" class="modal d-block" tabindex="-1">
      <div class="modal-dialog modal-lg modal-dialog-centered modal-dialog-scrollable">
        <div class="modal-content">
          <form @submit.prevent="saveProduct">
            <div class="modal-header">
              <h5 class="modal-title">{{ editingId == null ? "Add Product" : "Edit Product" }}</h5>
              <button type="button" class="btn-close" @click="showModal = false"></button>
            </div>
            <div class="modal-body">
              <div v-if="productError" class="alert alert-danger py-2 small" role="alert">
                <i class="bi bi-exclamation-triangle me-1"></i>{{ productError }}
              </div>
              <div class="row g-3">
                <div class="col-12">
                  <label class="form-label" for="p-name">Name *</label>
                  <input id="p-name" v-model="form.name" class="form-control" type="text" autofocus />
                </div>
                <div class="col-md-6">
                  <label class="form-label" for="p-sku">SKU</label>
                  <input id="p-sku" v-model="form.sku" class="form-control" type="text" />
                </div>
                <div class="col-md-6">
                  <label class="form-label" for="p-barcode">Barcode</label>
                  <input id="p-barcode" v-model="form.barcode" class="form-control" type="text" />
                </div>
                <div class="col-12">
                  <label class="form-label" for="p-desc">Description</label>
                  <textarea
                    id="p-desc"
                    v-model="form.description"
                    class="form-control"
                    rows="2"
                  ></textarea>
                </div>
                <div class="col-md-6">
                  <label class="form-label" for="p-cat">Category</label>
                  <select id="p-cat" v-model="form.categoryId" class="form-select">
                    <option :value="null">None</option>
                    <option v-for="c in categories" :key="c.id" :value="c.id">{{ c.name }}</option>
                  </select>
                </div>
                <div class="col-md-6">
                  <label class="form-label" for="p-tax">Tax profile</label>
                  <select id="p-tax" v-model="form.taxProfileId" class="form-select">
                    <option :value="null">No tax</option>
                    <option v-for="t in taxProfiles" :key="t.id" :value="t.id">
                      {{ t.name }} ({{ t.rate }}%)
                    </option>
                  </select>
                </div>
                <div class="col-md-4">
                  <label class="form-label" for="p-cost">Cost price *</label>
                  <input
                    id="p-cost"
                    v-model.number="form.costPrice"
                    class="form-control"
                    type="number"
                    step="0.01"
                    min="0"
                  />
                </div>
                <div class="col-md-4">
                  <label class="form-label" for="p-sell">Sell price *</label>
                  <input
                    id="p-sell"
                    v-model.number="form.sellPrice"
                    class="form-control"
                    type="number"
                    step="0.01"
                    min="0"
                  />
                </div>
                <div class="col-md-4">
                  <label class="form-label" for="p-unit">Unit</label>
                  <input id="p-unit" v-model="form.unit" class="form-control" type="text" />
                </div>
                <div class="col-md-6">
                  <label class="form-label" for="p-reorder">Reorder level</label>
                  <input
                    id="p-reorder"
                    v-model.number="form.reorderLevel"
                    class="form-control"
                    type="number"
                    step="1"
                    min="0"
                  />
                </div>
                <div class="col-md-6 d-flex align-items-end">
                  <div class="form-check form-switch mb-2">
                    <input
                      id="p-active"
                      v-model="form.isActive"
                      class="form-check-input"
                      type="checkbox"
                      role="switch"
                    />
                    <label class="form-check-label" for="p-active">Product is active</label>
                  </div>
                </div>
              </div>
            </div>
            <div class="modal-footer">
              <button type="button" class="btn btn-outline-secondary" @click="showModal = false">
                Cancel
              </button>
              <button type="submit" class="btn btn-primary" :disabled="saving">
                <span v-if="saving" class="spinner-border spinner-border-sm me-2"></span>
                {{ editingId == null ? "Add" : "Save" }}
              </button>
            </div>
          </form>
        </div>
      </div>
    </div>

    <div v-if="showCatModal" class="modal-backdrop show"></div>
    <div v-if="showCatModal" class="modal d-block" tabindex="-1">
      <div class="modal-dialog modal-dialog-centered">
        <div class="modal-content">
          <form @submit.prevent="saveCategory">
            <div class="modal-header">
              <h5 class="modal-title">
                {{ catEditingId == null ? "Add Category" : "Edit Category" }}
              </h5>
              <button type="button" class="btn-close" @click="showCatModal = false"></button>
            </div>
            <div class="modal-body">
              <div v-if="categoryError" class="alert alert-danger py-2 small" role="alert">
                <i class="bi bi-exclamation-triangle me-1"></i>{{ categoryError }}
              </div>
              <div class="mb-3">
                <label class="form-label" for="cat-name">Name</label>
                <input id="cat-name" v-model="catForm.name" class="form-control" type="text" autofocus />
              </div>
              <div class="mb-0">
                <label class="form-label" for="cat-parent">
                  Parent category
                  <span class="text-muted fw-normal">(optional)</span>
                </label>
                <select id="cat-parent" v-model="catForm.parentId" class="form-select">
                  <option :value="null">None</option>
                  <option
                    v-for="c in categories.filter((x) => x.id !== catEditingId)"
                    :key="c.id"
                    :value="c.id"
                  >
                    {{ c.name }}
                  </option>
                </select>
              </div>
            </div>
            <div class="modal-footer">
              <button type="button" class="btn btn-outline-secondary" @click="showCatModal = false">
                Cancel
              </button>
              <button type="submit" class="btn btn-primary" :disabled="catSaving">
                <span v-if="catSaving" class="spinner-border spinner-border-sm me-2"></span>Save
              </button>
            </div>
          </form>
        </div>
      </div>
    </div>
    <div v-if="adjustTarget" class="modal-backdrop show"></div>
    <div v-if="adjustTarget" class="modal d-block" tabindex="-1">
      <div class="modal-dialog modal-dialog-centered">
        <div class="modal-content">
          <div class="modal-header">
            <h5 class="modal-title">Adjust stock — {{ adjustTarget.name }}</h5>
            <button type="button" class="btn-close" @click="adjustTarget = null"></button>
          </div>
          <div class="modal-body">
            <div v-if="adjustError" class="alert alert-danger py-2 small" role="alert">
              <i class="bi bi-exclamation-triangle me-1"></i>{{ adjustError }}
            </div>
            <div class="mb-2 text-muted small">
              Current stock: <strong>{{ adjustTarget.stock_qty }}</strong> {{ adjustTarget.unit }}
            </div>
            <div class="mb-3">
              <label class="form-label" for="adj-new">New stock total</label>
              <input
                id="adj-new"
                v-model.number="adjustNew"
                class="form-control"
                type="number"
                step="0.5"
                min="0"
              />
            </div>
            <div class="mb-0">
              <label class="form-label" for="adj-notes">Reason (optional)</label>
              <input
                id="adj-notes"
                v-model="adjustNotes"
                class="form-control"
                type="text"
                placeholder="e.g. stock count, damage, supplier sample…"
              />
            </div>
          </div>
          <div class="modal-footer">
            <button type="button" class="btn btn-outline-secondary" @click="adjustTarget = null">
              Cancel
            </button>
            <button type="button" class="btn btn-primary" :disabled="adjustSaving" @click="saveAdjust">
              <span v-if="adjustSaving" class="spinner-border spinner-border-sm me-2"></span>Save
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
