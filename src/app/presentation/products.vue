<template>
  <div>
    <div class="d-flex align-items-center justify-content-between mb-3">
      <h1 class="h4 mb-0">{{ t("products.title") }}</h1>
      <div class="d-flex gap-2">
        <button class="btn btn-outline-primary" type="button" @click="importCsv" :disabled="csvImporting">
          <i class="bi bi-filetype-csv me-1"></i>{{ t("products.importCsv") }}
        </button>
        <button class="btn btn-primary" type="button" @click="openAddProduct">
          <i class="bi bi-plus-lg me-1"></i>{{ t("products.addProduct") }}
        </button>
      </div>
    </div>

    <div v-if="csvResult" class="alert alert-info py-2 small" role="alert">
      <div class="d-flex justify-content-between align-items-start">
        <div>
          <i class="bi bi-file-earmark-arrow-up me-1"></i>
          <strong>{{ t("products.importResult", { imported: csvResult.imported }) }}</strong>
          <span v-if="csvResult.errors.length">
            {{ t("products.importRowsSkipped", { count: csvResult.errors.length }) }}
          </span>
          <ul v-if="csvResult.errors.length" class="mb-0 mt-1 small">
            <li v-for="(e, i) in csvResult.errors.slice(0, 10)" :key="i">
              {{ t("products.importRow", { row: e.row }) }}: {{ e.message }}
            </li>
            <li v-if="csvResult.errors.length > 10" class="text-muted">
              {{ t("products.andMore", { count: csvResult.errors.length - 10 }) }}
            </li>
          </ul>
        </div>
        <button type="button" class="btn-close" @click="csvResult = null"></button>
      </div>
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
          <i class="bi bi-plus-lg me-1"></i>{{ t("products.addCategory") }}
        </button>
        <div class="mt-2">
          <button
            class="cat-item"
            type="button"
            :class="{ active: selected == null }"
            @click="selected = null"
          >
            <i class="bi bi-box-seam"></i>
            <span>{{ t("products.allProducts") }}</span>
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
                :title="t('common.edit')"
                @click="openEditCategory(c)"
              >
                <i class="bi bi-pencil"></i>
              </button>
              <button
                class="btn btn-sm btn-link p-0 text-danger"
                type="button"
                :title="t('common.delete')"
                @click="removeCategory(c)"
              >
                <i class="bi bi-trash"></i>
              </button>
            </span>
          </button>
          <div v-if="!categories.length && !loading" class="text-muted small p-2">
            {{ t("products.noCategories") }}
          </div>
        </div>
      </div>

      <div class="card flex-grow-1">
        <div class="p-2 border-bottom d-flex gap-2">
          <input
            v-model="search"
            class="form-control form-control-sm"
            type="search"
            :placeholder="t('products.searchPlaceholder')"
          />
          <select
            v-model="statusFilter"
            class="form-select form-select-sm"
            style="width: auto"
            :aria-label="t('products.filterStatus')"
          >
            <option value="all">{{ t("products.allStatuses") }}</option>
            <option value="active">{{ t("common.active") }}</option>
            <option value="inactive">{{ t("common.inactive") }}</option>
          </select>
        </div>
        <div class="table-responsive">
          <table class="table align-middle mb-0">
            <thead>
              <tr>
                <th style="width: 52px"></th>
                <th>{{ t("common.name") }}</th>
                <th>{{ t("products.category") }}</th>
                <th>{{ t("products.skuBarcode") }}</th>
                <th class="text-end">{{ t("products.cost") }}</th>
                <th class="text-end">{{ t("products.sell") }}</th>
                <th class="text-end">{{ t("products.stock") }}</th>
                <th>{{ t("common.status") }}</th>
                <th class="text-end">{{ t("common.actions") }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-if="!filteredProducts.length">
                <td colspan="9" class="text-center text-muted py-4">
                  {{ t("products.noProducts") }}
                </td>
              </tr>
              <tr v-for="p in filteredProducts" :key="p.id">
                <td>
                  <img
                    v-if="p.image_path"
                    :src="convertFileSrc(p.image_path)"
                    class="product-thumb"
                    alt=""
                  />
                  <div v-else class="product-thumb product-thumb-empty">
                    <i class="bi bi-image"></i>
                  </div>
                </td>
                <td class="fw-semibold">{{ p.name }}</td>
                <td class="text-muted">{{ categoryName(p.category_id) }}</td>
                <td class="text-muted small">
                  <div v-if="p.sku">{{ t("products.sku") }}: {{ p.sku }}</div>
                  <div v-if="p.barcode">{{ p.barcode }}</div>
                  <span v-if="!p.sku && !p.barcode">—</span>
                </td>
                <td class="text-end">{{ fmt(p.cost_price) }}</td>
                <td class="text-end">{{ fmt(p.sell_price) }}</td>
                <td class="text-end">{{ p.stock_qty }} {{ p.unit }}</td>
                <td>
                  <div class="form-check form-switch mb-0">
                    <input
                      class="form-check-input"
                      type="checkbox"
                      role="switch"
                      :checked="p.is_active === 1"
                      :title="p.is_active ? t('products.activeTitle') : t('products.inactiveTitle')"
                      @change="toggleActive(p)"
                    />
                  </div>
                  <span
                    class="badge mt-1"
                    :class="p.is_active ? 'text-bg-success' : 'text-bg-secondary'"
                  >
                    {{ p.is_active ? t("common.active") : t("common.inactive") }}
                  </span>
                </td>
                <td class="text-end text-nowrap">
                  <button
                    class="btn btn-sm btn-outline-secondary me-1"
                    type="button"
                    :title="t('products.adjustStock')"
                    @click="openAdjust(p)"
                  >
                    <i class="bi bi-box-arrow-up-down me-1"></i>{{ t("products.stock") }}
                  </button>
                  <button
                    class="btn btn-sm btn-outline-primary me-1"
                    type="button"
                    :title="t('common.edit')"
                    @click="openEditProduct(p)"
                  >
                    <i class="bi bi-pencil-square"></i>
                  </button>
                  <button
                    class="btn btn-sm btn-outline-danger"
                    type="button"
                    :title="t('common.delete')"
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
              <h5 class="modal-title">
                {{ editingId == null ? t("products.addProductTitle") : t("products.editProductTitle") }}
              </h5>
              <button type="button" class="btn-close" @click="showModal = false"></button>
            </div>
            <div class="modal-body">
              <div v-if="productError" class="alert alert-danger py-2 small" role="alert">
                <i class="bi bi-exclamation-triangle me-1"></i>{{ productError }}
              </div>
              <div class="row g-3">
                <div class="col-12">
                  <label class="form-label" for="p-name">{{ t("common.name") }} *</label>
                  <input
                    id="p-name"
                    v-model="form.name"
                    class="form-control"
                    type="text"
                    autofocus
                    required
                  />
                </div>
                <div class="col-12">
                  <label class="form-label">
                    {{ t("products.image") }}
                    <span class="text-muted fw-normal">{{ t("common.optional") }}</span>
                  </label>
                  <div class="d-flex align-items-center gap-3">
                    <img
                      v-if="form.imagePath"
                      :src="convertFileSrc(form.imagePath)"
                      class="product-thumb product-thumb-lg"
                      alt=""
                    />
                    <div v-else class="product-thumb product-thumb-lg product-thumb-empty">
                      <i class="bi bi-image"></i>
                    </div>
                    <div class="d-flex flex-column gap-1">
                      <button
                        class="btn btn-sm btn-outline-secondary"
                        type="button"
                        @click="pickImage"
                      >
                        <i class="bi bi-folder2-open me-1"></i>
                        {{ form.imagePath || pendingImage ? t("products.chooseDifferent") : t("products.chooseImage") }}
                      </button>
                      <span v-if="pendingImage" class="text-muted small">
                        {{ t("products.imageWillAttach") }}
                      </span>
                      <button
                        v-if="form.imagePath || pendingImage"
                        class="btn btn-sm btn-outline-danger"
                        type="button"
                        @click="clearImage"
                      >
                        <i class="bi bi-x-lg me-1"></i>{{ t("common.remove") }}
                      </button>
                    </div>
                  </div>
                </div>
                <div class="col-md-6">
                  <label class="form-label" for="p-sku">{{ t("products.sku") }}</label>
                  <input id="p-sku" v-model="form.sku" class="form-control" type="text" />
                </div>
                <div class="col-md-6">
                  <label class="form-label" for="p-barcode">{{ t("products.barcode") }} *</label>
                  <input
                    id="p-barcode"
                    v-model="form.barcode"
                    class="form-control"
                    type="text"
                    required
                  />
                </div>
                <div class="col-12">
                  <label class="form-label" for="p-desc">{{ t("products.description") }}</label>
                  <textarea
                    id="p-desc"
                    v-model="form.description"
                    class="form-control"
                    rows="2"
                  ></textarea>
                </div>
                <div class="col-md-6">
                  <label class="form-label" for="p-cat">{{ t("products.category") }}</label>
                  <select id="p-cat" v-model="form.categoryId" class="form-select">
                    <option :value="null">{{ t("common.none") }}</option>
                    <option v-for="c in categories" :key="c.id" :value="c.id">{{ c.name }}</option>
                  </select>
                </div>
                <div class="col-md-6">
                  <label class="form-label" for="p-tax">{{ t("products.taxProfile") }}</label>
                  <select id="p-tax" v-model="form.taxProfileId" class="form-select">
                    <option :value="null">{{ t("products.noTax") }}</option>
                    <option v-for="t in taxProfiles" :key="t.id" :value="t.id">
                      {{ t.name }} ({{ t.rate }}%)
                    </option>
                  </select>
                </div>
                <div class="col-md-4">
                  <label class="form-label" for="p-cost">{{ t("products.costPrice") }} *</label>
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
                  <label class="form-label" for="p-sell">{{ t("products.sellPrice") }} *</label>
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
                  <label class="form-label" for="p-unit">{{ t("products.unit") }} *</label>
                  <input id="p-unit" v-model="form.unit" class="form-control" type="text" required />
                </div>
                <div class="col-md-6">
                  <label class="form-label" for="p-reorder">{{ t("products.reorderLevel") }}</label>
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
                    <label class="form-check-label" for="p-active">{{ t("products.productActive") }}</label>
                  </div>
                </div>
              </div>
            </div>
            <div class="modal-footer">
              <button type="button" class="btn btn-outline-secondary" @click="showModal = false">
                {{ t("common.cancel") }}
              </button>
              <button type="submit" class="btn btn-primary" :disabled="saving">
                <span v-if="saving" class="spinner-border spinner-border-sm me-2"></span>
                {{ editingId == null ? t("common.add") : t("common.save") }}
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
                {{ catEditingId == null ? t("products.addCategory") : t("products.editCategory") }}
              </h5>
              <button type="button" class="btn-close" @click="showCatModal = false"></button>
            </div>
            <div class="modal-body">
              <div v-if="categoryError" class="alert alert-danger py-2 small" role="alert">
                <i class="bi bi-exclamation-triangle me-1"></i>{{ categoryError }}
              </div>
              <div class="mb-3">
                <label class="form-label" for="cat-name">{{ t("common.name") }}</label>
                <input id="cat-name" v-model="catForm.name" class="form-control" type="text" autofocus />
              </div>
              <div class="mb-0">
                <label class="form-label" for="cat-parent">
                  {{ t("products.parentCategory") }}
                  <span class="text-muted fw-normal">{{ t("common.optional") }}</span>
                </label>
                <select id="cat-parent" v-model="catForm.parentId" class="form-select">
                  <option :value="null">{{ t("common.none") }}</option>
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
                {{ t("common.cancel") }}
              </button>
              <button type="submit" class="btn btn-primary" :disabled="catSaving">
                <span v-if="catSaving" class="spinner-border spinner-border-sm me-2"></span>{{ t("common.save") }}
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
            <h5 class="modal-title">{{ t("products.adjustStockTitle", { name: adjustTarget.name }) }}</h5>
            <button type="button" class="btn-close" @click="adjustTarget = null"></button>
          </div>
          <div class="modal-body">
            <div v-if="adjustError" class="alert alert-danger py-2 small" role="alert">
              <i class="bi bi-exclamation-triangle me-1"></i>{{ adjustError }}
            </div>
            <div class="mb-2 text-muted small">
              {{ t("products.currentStock") }}:
              <strong>{{ adjustTarget.stock_qty }}</strong> {{ adjustTarget.unit }}
            </div>
            <div class="mb-3">
              <label class="form-label" for="adj-new">{{ t("products.newStockTotal") }}</label>
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
              <label class="form-label" for="adj-notes">{{ t("products.reasonOptional") }}</label>
              <input
                id="adj-notes"
                v-model="adjustNotes"
                class="form-control"
                type="text"
                :placeholder="t('products.adjustReasonPlaceholder')"
              />
            </div>
          </div>
          <div class="modal-footer">
            <button type="button" class="btn btn-outline-secondary" @click="adjustTarget = null">
              {{ t("common.cancel") }}
            </button>
            <button type="button" class="btn btn-primary" :disabled="adjustSaving" @click="saveAdjust">
              <span v-if="adjustSaving" class="spinner-border spinner-border-sm me-2"></span>{{ t("common.save") }}
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { useI18n } from "vue-i18n";
import { useCatalogStore } from "../../stores/catalog";
import { useSettingsStore } from "../../stores/settings";
import { useAuth } from "../../composables/useAuth";
import { select } from "../../lib/db";
import type { Category, Product, TaxProfile } from "../../types";

const catalog = useCatalogStore();
const settings = useSettingsStore();
const auth = useAuth();
const { t, locale } = useI18n();

const categories = ref<Category[]>([]);
const selected = ref<number | null>(null);
const search = ref("");
const statusFilter = ref<"all" | "active" | "inactive">("all");
const loading = ref(false);
const error = ref("");
const notice = ref("");

const showModal = ref(false);
const saving = ref(false);
const productError = ref("");
const editingId = ref<number | null>(null);
const pendingImage = ref<string | null>(null);
const form = ref({
  name: "",
  sku: "",
  barcode: "",
  description: "",
  categoryId: null as number | null,
  costPrice: 0,
  sellPrice: 0,
  taxProfileId: null as number | null,
  unit: "store item",
  reorderLevel: 0,
  isActive: true,
  imagePath: null as string | null,
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

const csvImporting = ref(false);
const csvResult = ref<{ imported: number; errors: { row: number; message: string }[] } | null>(
  null,
);

async function importCsv() {
  const filePath = await open({
    multiple: false,
    filters: [{ name: "CSV", extensions: ["csv"] }],
  });
  if (typeof filePath !== "string") return;
  csvImporting.value = true;
  csvResult.value = null;
  error.value = "";
  try {
    csvResult.value = await invoke("import_products_csv", { sourcePath: filePath });
    await Promise.all([loadCategories(), catalog.load()]);
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    csvImporting.value = false;
  }
}

function fmt(n: number): string {
  if (!settings.currency) return n.toFixed(2);
  return new Intl.NumberFormat(locale.value, {
    style: "currency",
    currency: settings.currency,
    currencyDisplay: "narrowSymbol",
  }).format(n);
}

const filteredProducts = computed(() => {
  const q = search.value.trim().toLowerCase();
  return catalog.products.filter((p) => {
    if (selected.value != null && p.category_id !== selected.value) return false;
    if (statusFilter.value === "active" && p.is_active !== 1) return false;
    if (statusFilter.value === "inactive" && p.is_active === 1) return false;
    if (!q) return true;
    return (
      p.name.toLowerCase().includes(q) ||
      (p.sku?.toLowerCase().includes(q) ?? false) ||
      (p.barcode?.toLowerCase().includes(q) ?? false)
    );
  });
});

async function toggleActive(p: Product) {
  const next = p.is_active === 1 ? 0 : 1;
  try {
    await invoke("set_product_active", { productId: p.id, isActive: next === 1 });
    p.is_active = next;
    notice.value =
      next === 1
        ? t("products.activated", { name: p.name })
        : t("products.deactivated", { name: p.name });
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  }
}

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
    categoryError.value = t("products.catNameRequired");
    return;
  }
  catSaving.value = true;
  try {
    if (catEditingId.value == null) {
      await invoke<number>("create_category", {
        name: catForm.value.name,
        parentId: catForm.value.parentId,
      });
      notice.value = t("products.categoryAdded");
    } else {
      await invoke("update_category", {
        categoryId: catEditingId.value,
        name: catForm.value.name,
        parentId: catForm.value.parentId,
      });
      notice.value = t("products.categoryUpdated");
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
  if (!window.confirm(t("products.deleteCategoryConfirm", { name: c.name }))) return;
  try {
    await invoke("delete_category", { categoryId: c.id });
    notice.value = t("products.categoryDeleted", { name: c.name });
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
  pendingImage.value = null;
  form.value = {
    name: "",
    sku: "",
    barcode: "",
    description: "",
    categoryId: selected.value,
    costPrice: 0,
    sellPrice: 0,
    taxProfileId: null,
    unit: "store item",
    reorderLevel: 0,
    isActive: true,
    imagePath: null,
  };
  showModal.value = true;
}

function openEditProduct(p: Product) {
  productError.value = "";
  editingId.value = p.id;
  pendingImage.value = null;
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
    imagePath: p.image_path,
  };
  showModal.value = true;
}

function clearImage() {
  form.value.imagePath = null;
  pendingImage.value = null;
}

async function pickImage() {
  productError.value = "";
  const selectedPath = await open({
    multiple: false,
    filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg", "gif", "webp", "bmp"] }],
  });
  if (typeof selectedPath !== "string") return;
  if (editingId.value != null) {
    try {
      form.value.imagePath = await invoke<string>("import_product_image", {
        productId: editingId.value,
        sourcePath: selectedPath,
      });
      pendingImage.value = null;
    } catch (e) {
      productError.value = e instanceof Error ? e.message : String(e);
    }
  } else {
    pendingImage.value = selectedPath;
    form.value.imagePath = null;
  }
}

function validateProduct(): string {
  if (!form.value.name.trim()) return t("products.nameRequired");
  if (!form.value.barcode.trim()) return t("products.barcodeRequired");
  const cost = form.value.costPrice;
  if (typeof cost !== "number" || isNaN(cost) || cost < 0)
    return t("products.costInvalid");
  const sell = form.value.sellPrice;
  if (typeof sell !== "number" || isNaN(sell) || sell < 0)
    return t("products.sellInvalid");
  if (sell <= cost) return t("products.sellAboveCost");
  if (!form.value.unit.trim()) return t("products.unitRequired");
  const reorder = form.value.reorderLevel;
  if (typeof reorder !== "number" || isNaN(reorder) || reorder < 0)
    return t("products.reorderInvalid");
  return "";
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
    imagePath: form.value.imagePath,
    isActive: form.value.isActive,
  };
}

async function saveProduct() {
  productError.value = "";
  const err = validateProduct();
  if (err) {
    productError.value = err;
    return;
  }
  saving.value = true;
  try {
    let id = editingId.value;
    if (id == null) {
      id = await invoke<number>("create_product", { input: productPayload() });
      notice.value = t("products.productAdded");
    } else {
      await invoke("update_product", { productId: id, input: productPayload() });
      notice.value = t("products.productUpdated");
    }
    if (pendingImage.value) {
      await invoke<string>("import_product_image", {
        productId: id,
        sourcePath: pendingImage.value,
      });
      pendingImage.value = null;
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
  if (!window.confirm(t("products.deleteProductConfirm", { name: p.name }))) return;
  try {
    await invoke("delete_product", { productId: p.id });
    notice.value = t("products.productDeleted", { name: p.name });
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
      userId: auth.user?.id ?? null,
    });
    adjustTarget.value = null;
    notice.value = t("products.stockUpdated", {
      from: target.stock_qty,
      to: adjustNew.value,
      unit: target.unit,
    });
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
