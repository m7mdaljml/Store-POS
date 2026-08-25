<template>
  <div>
    <div class="d-flex align-items-center justify-content-between mb-3">
      <h1 class="h4 mb-0">{{ t("products.title") }}</h1>
      <div class="d-flex gap-2">
        <AsyncButton
          variant="outline-primary"
          :loading="csvImporting"
          @click="importCsv"
        >
          <i v-if="!csvImporting" class="bi bi-filetype-csv mx-1"></i
          >{{ t("products.importCsv") }}
        </AsyncButton>
        <button class="btn btn-primary" type="button" @click="openAddProduct">
          <i class="bi bi-plus-lg mx-1"></i>{{ t("products.addProduct") }}
        </button>
      </div>
    </div>

    <div v-if="csvResult" class="alert alert-info py-2 small" role="alert">
      <div class="d-flex justify-content-between align-items-start">
        <div>
          <i class="bi bi-file-earmark-arrow-up mx-1"></i>
          <strong>{{
            t("products.importResult", { imported: csvResult.imported })
          }}</strong>
          <span v-if="csvResult.errors.length">
            {{
              t("products.importRowsSkipped", {
                count: csvResult.errors.length,
              })
            }}
          </span>
          <ul v-if="csvResult.errors.length" class="mb-0 mt-1 small">
            <li v-for="(e, i) in csvResult.errors.slice(0, 10)" :key="i">
              {{ t("products.importRow", { row: e.row }) }}: {{ e.message }}
            </li>
            <li v-if="csvResult.errors.length > 10" class="text-muted">
              {{
                t("products.andMore", { count: csvResult.errors.length - 10 })
              }}
            </li>
          </ul>
        </div>
        <button
          type="button"
          class="btn-close"
          @click="csvResult = null"
        ></button>
      </div>
    </div>

    <div class="d-flex gap-3 align-items-start">
      <div class="card category-sidebar">
        <button
          class="btn btn-sm btn-primary w-100"
          type="button"
          @click="openAddCategory"
        >
          <i class="bi bi-plus-lg mx-1"></i>{{ t("products.addCategory") }}
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
            <span class="badge text-bg-secondary">{{
              catalog.products.length
            }}</span>
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
                class="btn btn-sm btn-link p-0 mx-1"
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
          <div
            v-if="!categories.length && !loading"
            class="text-muted small p-2"
          >
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
            <thead v-if="visibleProducts.length">
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
            <tbody v-if="loading">
              <tr v-for="i in 6" :key="i">
                <td colspan="9" class="py-2">
                  <div
                    class="skeleton"
                    style="height: 0.8rem"
                    :style="{ width: 96 - (i % 3) * 5 + '%' }"
                  ></div>
                </td>
              </tr>
            </tbody>
            <tbody v-else-if="!visibleProducts.length">
              <tr>
                <td colspan="9" class="p-0 border-0">
                  <EmptyState
                    :image="emptyProducts"
                    :message="
                      search
                        ? t('products.noMatchingProducts')
                        : t('products.noProducts')
                    "
                  />
                </td>
              </tr>
            </tbody>
            <tbody v-for="p in visibleProducts" :key="p.id">
              <tr>
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
                <td class="fw-semibold">
                  {{ p.name }}
                  <span
                    v-if="p.is_quick"
                    class="badge text-bg-warning ms-1"
                    :title="t('products.quickItem')"
                    >{{ t("products.quickItem") }}</span
                  >
                </td>
                <td class="text-muted">{{ categoryName(p.category_id) }}</td>
                <td class="text-muted small">
                  <div v-if="p.barcode">{{ p.barcode }}</div>
                  <span v-if="!p.barcode">—</span>
                </td>
                <td class="text-end">{{ fmt(p.cost_price) }}</td>
                <td class="text-end">{{ fmt(p.sell_price) }}</td>
                <td class="text-end">{{ p.stock_qty }} {{ p.unit }}</td>
                <td>
                  <span
                    class="badge mt-1"
                    :class="
                      p.is_active ? 'text-bg-success' : 'text-bg-secondary'
                    "
                  >
                    {{
                      p.is_active ? t("common.active") : t("common.inactive")
                    }}
                  </span>
                </td>
                <td class="text-end text-nowrap">
                  <button
                    class="btn btn-sm btn-outline-secondary mx-1"
                    type="button"
                    :title="t('products.adjustStock')"
                    @click="openAdjust(p)"
                  >
                    <i class="bi bi-box-arrow-up-down mx-1"></i
                    >{{ t("products.stock") }}
                  </button>
                  <button
                    class="btn btn-sm btn-outline-primary mx-1"
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
        <Paginator
          v-model:page="productPage"
          :total-items="filteredProducts.length"
          :page-size="settings.pageSize"
        />
      </div>
    </div>

    <div v-if="showModal" class="modal-backdrop show"></div>
    <div v-if="showModal" class="modal d-block" tabindex="-1">
      <div
        class="modal-dialog modal-lg modal-dialog-centered modal-dialog-scrollable"
      >
        <div class="modal-content">
          <form novalidate @submit.prevent="saveProduct">
            <div class="modal-header">
              <h5 class="modal-title">
                {{
                  editingId == null
                    ? t("products.addProductTitle")
                    : t("products.editProductTitle")
                }}
              </h5>
              <button
                type="button"
                class="btn-close"
                @click="showModal = false"
              ></button>
            </div>
            <div class="modal-body">
              <div
                v-if="productError"
                class="alert alert-danger py-2 small"
                role="alert"
              >
                <i class="bi bi-exclamation-triangle mx-1"></i
                >{{ productError }}
              </div>
              <div class="row g-3">
                <div class="col-12">
                  <label class="form-label" for="p-name"
                    >{{ t("common.name") }} *</label
                  >
                  <input
                    id="p-name"
                    v-model="form.name"
                    class="form-control"
                    :class="{ 'is-invalid': errors.name }"
                    type="text"
                    autofocus
                    @input="clearFieldError(errors, 'name')"
                  />
                  <div class="invalid-feedback">{{ errors.name }}</div>
                </div>
                <div class="col-12">
                  <label class="form-label">
                    {{ t("products.image") }}
                    <span class="text-muted fw-normal">{{
                      t("common.optional")
                    }}</span>
                  </label>
                  <div class="d-flex align-items-center gap-3">
                    <img
                      v-if="form.imagePath"
                      :src="convertFileSrc(form.imagePath)"
                      class="product-thumb product-thumb-lg"
                      alt=""
                    />
                    <div
                      v-else
                      class="product-thumb product-thumb-lg product-thumb-empty"
                    >
                      <i class="bi bi-image"></i>
                    </div>
                    <div class="d-flex flex-column gap-1">
                      <button
                        class="btn btn-sm btn-outline-secondary"
                        type="button"
                        @click="pickImage"
                      >
                        <i class="bi bi-folder2-open mx-1"></i>
                        {{
                          form.imagePath || pendingImage
                            ? t("products.chooseDifferent")
                            : t("products.chooseImage")
                        }}
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
                        <i class="bi bi-x-lg mx-1"></i>{{ t("common.remove") }}
                      </button>
                    </div>
                  </div>
                </div>

                <div class="col-md-12">
                  <label class="form-label" for="p-barcode"
                    >{{ t("products.barcode") }} *</label
                  >
                  <input
                    id="p-barcode"
                    v-model="form.barcode"
                    class="form-control"
                    :class="{ 'is-invalid': errors.barcode }"
                    type="text"
                    inputmode="numeric"
                    pattern="[0-9]*"
                    @input="onBarcodeInput"
                  />
                  <div class="invalid-feedback">{{ errors.barcode }}</div>
                </div>
                <div class="col-12">
                  <label class="form-label" for="p-desc">{{
                    t("products.description")
                  }}</label>
                  <textarea
                    id="p-desc"
                    v-model="form.description"
                    class="form-control"
                    rows="2"
                  ></textarea>
                </div>
                <div class="col-md-6">
                  <label class="form-label" for="p-cat">{{
                    t("products.category")
                  }}</label>
                  <AppSelect
                    id="p-cat"
                    v-model="form.categoryId"
                    :items="categoryFormOptions"
                    :option-label="(c) => c.name"
                    :option-value="(c) => c.id"
                  />
                </div>
                <div class="col-md-6">
                  <label class="form-label" for="p-tax">{{
                    t("products.taxProfile")
                  }}</label>
                  <AppSelect
                    id="p-tax"
                    v-model="form.taxProfileId"
                    :items="taxProfileOptions"
                    :option-label="(tp) => tp.label"
                    :option-value="(tp) => tp.id"
                  />
                </div>
                <div class="col-md-4">
                  <label class="form-label" for="p-cost"
                    >{{ t("products.costPrice") }} *</label
                  >
                  <input
                    id="p-cost"
                    v-model.number="form.costPrice"
                    class="form-control"
                    :class="{ 'is-invalid': errors.costPrice || productError }"
                    type="number"
                    step="1"
                    min="0"
                    @input="clearFieldError(errors, 'costPrice')"
                  />
                  <div class="invalid-feedback">{{ errors.costPrice }}</div>
                </div>
                <div class="col-md-4">
                  <label class="form-label" for="p-sell"
                    >{{ t("products.sellPrice") }} *</label
                  >
                  <input
                    id="p-sell"
                    v-model.number="form.sellPrice"
                    class="form-control"
                    :class="{ 'is-invalid': errors.sellPrice }"
                    type="number"
                    step="1"
                    min="0"
                    @input="clearFieldError(errors, 'sellPrice')"
                  />
                  <div class="invalid-feedback">
                    {{ errors.sellPrice || errors.relation }}
                  </div>
                </div>
                <div class="col-md-4">
                  <label class="form-label" for="p-unit"
                    >{{ t("products.unit") }} *</label
                  >
                  <input
                    id="p-unit"
                    v-model="form.unit"
                    class="form-control"
                    :class="{ 'is-invalid': errors.unit }"
                    type="text"
                    @input="clearFieldError(errors, 'unit')"
                  />
                  <div class="invalid-feedback">{{ errors.unit }}</div>
                </div>
                <div class="col-md-6">
                  <label class="form-label" for="p-reorder">{{
                    t("products.reorderLevel")
                  }}</label>
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
                  <div class="d-flex align-items-center gap-4 mb-2 flex-wrap">
                    <div class="form-check form-switch">
                      <input
                        id="p-active"
                        v-model="form.isActive"
                        class="form-check-input"
                        type="checkbox"
                        role="switch"
                      />
                      <label class="form-check-label" for="p-active">{{
                        t("products.productActive")
                      }}</label>
                    </div>
                    <div class="form-check form-switch">
                      <input
                        id="p-quick"
                        v-model="form.isQuick"
                        class="form-check-input"
                        type="checkbox"
                        role="switch"
                      />
                      <label class="form-check-label" for="p-quick">{{
                        t("products.quickItem")
                      }}</label>
                    </div>
                  </div>
                </div>
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

    <div v-if="showCatModal" class="modal-backdrop show"></div>
    <div v-if="showCatModal" class="modal d-block" tabindex="-1">
      <div class="modal-dialog modal-dialog-centered">
        <div class="modal-content">
          <form novalidate @submit.prevent="saveCategory">
            <div class="modal-header">
              <h5 class="modal-title">
                {{
                  catEditingId == null
                    ? t("products.addCategory")
                    : t("products.editCategory")
                }}
              </h5>
              <button
                type="button"
                class="btn-close"
                @click="showCatModal = false"
              ></button>
            </div>
            <div class="modal-body">
              <div
                v-if="categoryError"
                class="alert alert-danger py-2 small"
                role="alert"
              >
                <i class="bi bi-exclamation-triangle mx-1"></i
                >{{ categoryError }}
              </div>
              <div class="mb-3">
                <label class="form-label" for="cat-name">{{
                  t("common.name")
                }}</label>
                <input
                  id="cat-name"
                  v-model="catForm.name"
                  class="form-control"
                  :class="{ 'is-invalid': catErrors.name }"
                  type="text"
                  autofocus
                  @input="clearFieldError(catErrors, 'name')"
                />
                <div class="invalid-feedback">{{ catErrors.name }}</div>
              </div>
              <div class="mb-0">
                <label class="form-label" for="cat-parent">
                  {{ t("products.parentCategory") }}
                  <span class="text-muted fw-normal">{{
                    t("common.optional")
                  }}</span>
                </label>
                <AppSelect
                  id="cat-parent"
                  v-model="catForm.parentId"
                  :items="parentCategoryOptions"
                  :option-label="(c) => c.name"
                  :option-value="(c) => c.id"
                />
              </div>
              <div
                class="d-flex justify-content-end gap-2 mt-3 pt-3 border-top"
              >
                <button
                  type="button"
                  class="btn btn-outline-secondary"
                  @click="showCatModal = false"
                >
                  {{ t("common.cancel") }}
                </button>
                <AsyncButton
                  type="submit"
                  :loading="catSaving"
                  :disabled="!canSaveCat"
                >
                  {{ t("common.save") }}
                </AsyncButton>
              </div>
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
            <h5 class="modal-title">
              {{ t("products.adjustStockTitle", { name: adjustTarget.name }) }}
            </h5>
            <button
              type="button"
              class="btn-close"
              @click="adjustTarget = null"
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
            <div class="mb-2 text-muted small">
              {{ t("products.currentStock") }}:
              <strong>{{ adjustTarget.stock_qty }}</strong>
              {{ adjustTarget.unit }}
            </div>
            <div class="mb-3">
              <label class="form-label" for="adj-new">{{
                t("products.newStockTotal")
              }}</label>
              <input
                id="adj-new"
                v-model.number="adjustNew"
                class="form-control"
                type="number"
                step="1"
                min="0"
              />
            </div>
            <div class="mb-0">
              <label class="form-label" for="adj-notes">{{
                t("products.reasonOptional")
              }}</label>
              <input
                id="adj-notes"
                v-model="adjustNotes"
                class="form-control"
                type="text"
                :placeholder="t('products.adjustReasonPlaceholder')"
              />
            </div>
            <div class="d-flex justify-content-end gap-2 mt-3 pt-3 border-top">
              <button
                type="button"
                class="btn btn-outline-secondary"
                @click="adjustTarget = null"
              >
                {{ t("common.cancel") }}
              </button>
              <AsyncButton
                :loading="adjustSaving"
                :disabled="!canAdjust"
                @click="saveAdjust"
              >
                {{ t("common.save") }}
              </AsyncButton>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from "vue";
import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { useI18n } from "vue-i18n";
import EmptyState from "../../components/EmptyState.vue";
import AsyncButton from "../../components/AsyncButton.vue";
import Paginator from "../../components/Paginator.vue";
import AppSelect from "../../components/AppSelect.vue";
import {
  applyFieldRules,
  clearFieldError,
  useFormGuard,
} from "../../composables/useFormGuard";
import { useToast } from "../../composables/useToast";
import { useConfirm } from "../../composables/useConfirm";
import { useCatalogStore } from "../../stores/catalog";
import { useSettingsStore } from "../../stores/settings";
import { useAuth } from "../../composables/useAuth";
import { select } from "../../lib/db";
import { formatMoney } from "../../lib/currency";
import type { Category, Product, TaxProfile } from "../../types";
import emptyProducts from "../../assets/empty/products.svg";

const catalog = useCatalogStore();
const settings = useSettingsStore();
const auth = useAuth();
const toast = useToast();
const { confirmDialog } = useConfirm();
const { t } = useI18n();

const categories = ref<Category[]>([]);
const selected = ref<number | null>(null);
const search = ref("");
const statusFilter = ref<"all" | "active" | "inactive">("all");
const loading = ref(false);

const showModal = ref(false);
const saving = ref(false);
const productError = ref("");
const editingId = ref<number | null>(null);
const pendingImage = ref<string | null>(null);
const form = ref({
  name: "",
  barcode: "",
  description: "",
  categoryId: null as number | null,
  costPrice: 0,
  sellPrice: 0,
  taxProfileId: null as number | null,
  unit: "store item",
  reorderLevel: 0,
  isActive: true,
  isQuick: false,
  imagePath: null as string | null,
});
const errors = reactive<Record<string, string>>({});
const guard = useFormGuard(form);
const canSave = computed(() => guard.isDirty.value && !saving.value);

function resetErrors() {
  for (const key of Object.keys(errors)) delete errors[key];
}

function resetCatErrors() {
  for (const key of Object.keys(catErrors)) delete catErrors[key];
}

const showCatModal = ref(false);
const catSaving = ref(false);
const categoryError = ref("");
const catEditingId = ref<number | null>(null);
const catForm = ref({ name: "", parentId: null as number | null });
const catErrors = reactive<Record<string, string>>({});
const catGuard = useFormGuard(catForm);
const canSaveCat = computed(() => catGuard.isDirty.value && !catSaving.value);

const adjustTarget = ref<Product | null>(null);
const adjustNew = ref(0);
const adjustNotes = ref("");
const adjustSaving = ref(false);
const adjustError = ref("");

const adjustForm = computed(() => ({
  targetId: adjustTarget.value?.id ?? null,
  newQty: adjustNew.value,
  notes: adjustNotes.value,
}));
const adjustGuard = useFormGuard(adjustForm);
const canAdjust = computed(
  () => adjustGuard.isDirty.value && !adjustSaving.value,
);

const taxProfiles = ref<TaxProfile[]>([]);

/** Dropdown option lists: a leading "None" entry plus the DB rows. */
const categoryFormOptions = computed<{ id: number | null; name: string }[]>(
  () => [
    { id: null, name: t("common.none") },
    ...categories.value.map((c) => ({ id: c.id, name: c.name })),
  ],
);
const parentCategoryOptions = computed<{ id: number | null; name: string }[]>(
  () => [
    { id: null, name: t("common.none") },
    ...categories.value
      .filter((x) => x.id !== catEditingId.value)
      .map((c) => ({ id: c.id, name: c.name })),
  ],
);
const taxProfileOptions = computed<{ id: number | null; label: string }[]>(() => [
  { id: null, label: t("products.noTax") },
  ...taxProfiles.value.map((tp) => ({
    id: tp.id,
    label: `${tp.name} (${tp.rate}%)`,
  })),
]);

const csvImporting = ref(false);
const csvResult = ref<{
  imported: number;
  errors: { row: number; message: string }[];
} | null>(null);

async function importCsv() {
  const filePath = await open({
    multiple: false,
    filters: [{ name: "CSV", extensions: ["csv"] }],
  });
  if (typeof filePath !== "string") return;
  csvImporting.value = true;
  csvResult.value = null;
  try {
    csvResult.value = await invoke("import_products_csv", {
      sourcePath: filePath,
      userId: auth.user?.id ?? null,
    });
    await Promise.all([loadCategories(), catalog.load()]);
  } catch (e) {
    toast.error(e instanceof Error ? e.message : String(e));
  } finally {
    csvImporting.value = false;
  }
}

function fmt(n: number): string {
  return formatMoney(n);
}

const filteredProducts = computed(() => {
  const q = search.value.trim().toLowerCase();
  return catalog.products.filter((p) => {
    if (selected.value != null && p.category_id !== selected.value)
      return false;
    if (statusFilter.value === "active" && p.is_active !== 1) return false;
    if (statusFilter.value === "inactive" && p.is_active === 1) return false;
    if (!q) return true;
    return (
      p.name.toLowerCase().includes(q) ||
      (p.barcode?.toLowerCase().includes(q) ?? false)
    );
  });
});

// Client-side windowing: the catalog store is shared with checkout/purchases,
// so this list is paginated in the UI instead of via SQL limits.
const productPage = ref(1);
const visibleProducts = computed(() => {
  const size = settings.pageSize;
  const start = (productPage.value - 1) * size;
  return filteredProducts.value.slice(start, start + size);
});

watch([search, statusFilter, selected, () => settings.pageSize], () => {
  productPage.value = 1;
});

const categoryName = (id: number | null) =>
  categories.value.find((c) => c.id === id)?.name ?? "—";

async function loadCategories() {
  loading.value = true;
  try {
    categories.value = await invoke<Category[]>("list_categories");
  } catch (e) {
    toast.error(e instanceof Error ? e.message : String(e));
  } finally {
    loading.value = false;
  }
}

/* ----- Categories ----- */

function openAddCategory() {
  categoryError.value = "";
  catEditingId.value = null;
  catForm.value = { name: "", parentId: null };
  resetCatErrors();
  catGuard.capture();
  showCatModal.value = true;
}

function openEditCategory(c: Category) {
  categoryError.value = "";
  catEditingId.value = c.id;
  catForm.value = { name: c.name, parentId: c.parentId ?? null };
  resetCatErrors();
  catGuard.capture();
  showCatModal.value = true;
}

async function saveCategory() {
  categoryError.value = "";
  if (
    !applyFieldRules(catErrors, [
      ["name", !!catForm.value.name.trim(), t("common.name")],
    ])
  ) {
    toast.error(t("common.fixErrors"));
    return;
  }
  catSaving.value = true;
  try {
    if (catEditingId.value == null) {
      await invoke<number>("create_category", {
        name: catForm.value.name,
        parentId: catForm.value.parentId,
        userId: auth.user?.id ?? null,
      });
      toast.success(t("products.categoryAdded"));
    } else {
      await invoke("update_category", {
        categoryId: catEditingId.value,
        name: catForm.value.name,
        parentId: catForm.value.parentId,
        userId: auth.user?.id ?? null,
      });
      toast.success(t("products.categoryUpdated"));
    }
    catGuard.markSaved();
    showCatModal.value = false;
    await Promise.all([loadCategories(), catalog.load()]);
  } catch (e) {
    categoryError.value = e instanceof Error ? e.message : String(e);
  } finally {
    catSaving.value = false;
  }
}

async function removeCategory(c: Category) {
  if (
    !(await confirmDialog({
      message: t("products.deleteCategoryConfirm", { name: c.name }),
    }))
  )
    return;
  try {
    await invoke("delete_category", { categoryId: c.id, userId: auth.user?.id ?? null });
    toast.success(t("products.categoryDeleted", { name: c.name }));
    if (selected.value === c.id) selected.value = null;
    await Promise.all([loadCategories(), catalog.load()]);
  } catch (e) {
    toast.error(e instanceof Error ? e.message : String(e));
  }
}

/* ----- Products ----- */

function openAddProduct() {
  productError.value = "";
  editingId.value = null;
  pendingImage.value = null;
  form.value = {
    name: "",
    barcode: "",
    description: "",
    categoryId: selected.value,
    costPrice: 0,
    sellPrice: 0,
    taxProfileId: null,
    unit: "store item",
    reorderLevel: 0,
    isActive: true,
    isQuick: false,
    imagePath: null,
  };
  resetErrors();
  guard.capture();
  showModal.value = true;
}

function openEditProduct(p: Product) {
  productError.value = "";
  editingId.value = p.id;
  pendingImage.value = null;
  form.value = {
    name: p.name,
    barcode: p.barcode ?? "",
    description: p.description ?? "",
    categoryId: p.category_id,
    costPrice: p.cost_price,
    sellPrice: p.sell_price,
    taxProfileId: p.tax_profile_id,
    unit: p.unit,
    reorderLevel: p.reorder_level,
    isActive: p.is_active === 1,
    isQuick: p.is_quick === 1,
    imagePath: p.image_path,
  };
  resetErrors();
  guard.capture();
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
    filters: [
      {
        name: "Images",
        extensions: ["png", "jpg", "jpeg", "gif", "webp", "bmp"],
      },
    ],
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

function onBarcodeInput(e: Event) {
  const input = e.target as HTMLInputElement;
  input.value = input.value.replace(/\D/g, "");
  form.value.barcode = input.value;
  clearFieldError(errors, "barcode");
}

function validateProduct(): boolean {
  const cost = form.value.costPrice;
  const sell = form.value.sellPrice;
  const reorder = form.value.reorderLevel;
  const ok = applyFieldRules(errors, [
    ["name", !!form.value.name.trim(), t("common.name")],
    [
      "barcode",
      !!form.value.barcode.trim() && /^\d+$/.test(form.value.barcode.trim()),
      t("products.barcodeDigits"),
    ],
    [
      "costPrice",
      typeof cost === "number" && !isNaN(cost) && cost > 0,
      t("products.costInvalid"),
    ],
    [
      "sellPrice",
      typeof sell === "number" && !isNaN(sell) && sell >= 0,
      t("products.sellPrice"),
    ],
    ["unit", !!form.value.unit.trim(), t("products.unit")],
    [
      "reorderLevel",
      typeof reorder === "number" && !isNaN(reorder) && reorder >= 0,
      t("products.reorderLevel"),
    ],
  ]);
  if (typeof cost === "number" && typeof sell === "number" && sell <= cost) {
    productError.value = t("products.sellAboveCost");
  }
  return ok && !productError.value;
}

function productPayload() {
  return {
    name: form.value.name,
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
    isQuick: form.value.isQuick,
  };
}

async function saveProduct() {
  productError.value = "";
  if (!validateProduct()) {
    toast.error(t("common.fixErrors"));
    return;
  }
  saving.value = true;
  try {
    let id = editingId.value;
    if (id == null) {
      id = await invoke<number>("create_product", { input: productPayload(), userId: auth.user?.id ?? null });
      toast.success(t("products.productAdded"));
    } else {
      await invoke("update_product", {
        productId: id,
        input: productPayload(),
        userId: auth.user?.id ?? null,
      });
      toast.success(t("products.productUpdated"));
    }
    if (pendingImage.value) {
      await invoke<string>("import_product_image", {
        productId: id,
        sourcePath: pendingImage.value,
      });
      pendingImage.value = null;
    }
    guard.markSaved();
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
  if (
    !(await confirmDialog({
      message: t("products.deleteProductConfirm", { name: p.name }),
    }))
  )
    return;
  try {
    await invoke("delete_product", { productId: p.id, userId: auth.user?.id ?? null });
    toast.success(t("products.productDeleted", { name: p.name }));
    await Promise.all([loadCategories(), catalog.load()]);
  } catch (e) {
    toast.error(e instanceof Error ? e.message : String(e));
  }
}

function openAdjust(p: Product) {
  adjustError.value = "";
  adjustTarget.value = p;
  adjustNew.value = p.stock_qty;
  adjustNotes.value = "";
  adjustGuard.capture();
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
    adjustGuard.markSaved();
    adjustTarget.value = null;
    toast.success(
      t("products.stockUpdated", {
        from: target.stock_qty,
        to: adjustNew.value,
        unit: target.unit,
      }),
    );
    await Promise.all([loadCategories(), catalog.load()]);
  } catch (e) {
    adjustError.value = e instanceof Error ? e.message : String(e);
  } finally {
    adjustSaving.value = false;
  }
}

onMounted(async () => {
  taxProfiles.value = await select<TaxProfile>(
    "SELECT id, name, rate FROM tax_profiles ORDER BY name",
  ).catch(() => []);
  await Promise.allSettled([loadCategories(), catalog.load(), settings.load()]);
});
</script>
