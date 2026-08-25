<template>
  <div>
    <div class="d-flex align-items-center justify-content-between mb-3">
      <h1 class="h4 mb-0">{{ t("suppliers.title") }}</h1>
      <button class="btn btn-primary" type="button" @click="openAdd">
        <i class="bi bi-plus-lg mx-1"></i>{{ t("suppliers.addSupplier") }}
      </button>
    </div>

    <div class="card">
      <div class="p-2 border-bottom">
        <input
          v-model="search"
          class="form-control form-control-sm"
          type="search"
          :placeholder="t('suppliers.searchPlaceholder')"
        />
      </div>
      <div class="table-responsive">
        <table class="table align-middle mb-0">
          <thead v-if="suppliers.length">
            <tr>
              <th>{{ t("suppliers.supplier") }}</th>
              <th>{{ t("suppliers.contact") }}</th>
              <th>{{ t("suppliers.phone") }}</th>
              <th>{{ t("suppliers.taxId") }}</th>
              <th class="text-end">{{ t("common.invoices") }}</th>
              <th class="text-end">{{ t("suppliers.totalPurchased") }}</th>
              <th class="text-end">{{ t("suppliers.amountDue") }}</th>
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
          <tbody v-else-if="!suppliers.length">
            <tr>
              <td colspan="8" class="p-0 border-0">
                <EmptyState
                  :image="emptySuppliers"
                  :message="
                    search
                      ? t('suppliers.noMatching')
                      : t('suppliers.noSuppliers')
                  "
                />
              </td>
            </tr>
          </tbody>
          <tbody v-for="s in suppliers" :key="s.id">
            <tr>
              <td class="fw-semibold">{{ s.name }}</td>
              <td class="text-muted">{{ s.contact ?? "—" }}</td>
              <td class="text-muted">{{ s.phone ?? "—" }}</td>
              <td class="text-muted">{{ s.tax_id ?? "—" }}</td>
              <td class="text-end">{{ s.invoice_count }}</td>
              <td class="text-end">{{ fmt(s.total_purchased) }}</td>
              <td class="text-end">
                <span v-if="s.total_due > 0" class="text-danger fw-semibold">
                  {{ fmt(s.total_due) }}
                </span>
                <span v-else class="text-muted">—</span>
              </td>
              <td class="text-end text-nowrap">
                <button
                  class="btn btn-sm btn-outline-secondary mx-1"
                  type="button"
                  :title="t('suppliers.viewDetailsTitle')"
                  @click="openDetail(s)"
                >
                  <i class="bi bi-eye"></i>
                </button>
                <button
                  class="btn btn-sm btn-outline-primary mx-1"
                  type="button"
                  :title="t('common.edit')"
                  @click="openEdit(s)"
                >
                  <i class="bi bi-pencil-square"></i>
                </button>
                <button
                  class="btn btn-sm btn-outline-danger"
                  type="button"
                  :title="t('common.delete')"
                  @click="remove(s)"
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
          <form @submit.prevent="save">
            <div class="modal-header">
              <h5 class="modal-title">
                {{
                  editingId == null
                    ? t("suppliers.addSupplierTitle")
                    : t("suppliers.editSupplierTitle")
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
                v-if="formError"
                class="alert alert-danger py-2 small"
                role="alert"
              >
                <i class="bi bi-exclamation-triangle mx-1"></i>{{ formError }}
              </div>
              <div class="mb-3">
                <label class="form-label" for="s-name">{{
                  t("suppliers.nameRequired")
                }}</label>
                <input
                  id="s-name"
                  v-model="form.name"
                  class="form-control"
                  :class="{ 'is-invalid': errors.name }"
                  type="text"
                  autofocus
                  @input="clearFieldError(errors, 'name')"
                />
                <div class="invalid-feedback">{{ errors.name }}</div>
              </div>
              <div class="mb-3">
                <label class="form-label" for="s-contact">{{
                  t("suppliers.contactPerson")
                }}</label>
                <input
                  id="s-contact"
                  v-model="form.contact"
                  class="form-control"
                  type="text"
                />
              </div>
              <div class="row g-3">
                <div class="col-md-6">
                  <label class="form-label" for="s-phone">{{
                    t("suppliers.phone")
                  }}</label>
                  <input
                    id="s-phone"
                    v-model="form.phone"
                    class="form-control"
                    type="text"
                  />
                </div>
                <div class="col-md-6">
                  <label class="form-label" for="s-email">{{
                    t("suppliers.email")
                  }}</label>
                  <input
                    id="s-email"
                    v-model="form.email"
                    class="form-control"
                    :class="{ 'is-invalid': errors.email }"
                    type="email"
                    @input="clearFieldError(errors, 'email')"
                  />
                  <div class="invalid-feedback">{{ errors.email }}</div>
                </div>
              </div>
              <div class="mt-3 mb-3">
                <label class="form-label" for="s-address">{{
                  t("suppliers.address")
                }}</label>
                <textarea
                  id="s-address"
                  v-model="form.address"
                  class="form-control"
                  rows="2"
                ></textarea>
              </div>
              <div class="mb-0">
                <label class="form-label" for="s-tax">{{
                  t("suppliers.taxId")
                }}</label>
                <input
                  id="s-tax"
                  v-model="form.taxId"
                  class="form-control"
                  type="text"
                />
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
              {{
                t("suppliers.supplierDetailTitle", {
                  name: detail?.supplier.name,
                })
              }}
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
            <div
              v-else-if="detailError"
              class="alert alert-danger py-2 small"
              role="alert"
            >
              <i class="bi bi-exclamation-triangle mx-1"></i>{{ detailError }}
            </div>
            <template v-else-if="detail">
              <div class="row g-3 mb-3">
                <div class="col-md-6">
                  <div class="detail-item">
                    <span class="detail-label">{{
                      t("suppliers.contact")
                    }}</span>
                    <span>{{ detail.supplier.contact ?? "—" }}</span>
                  </div>
                  <div class="detail-item">
                    <span class="detail-label">{{ t("suppliers.phone") }}</span>
                    <span>{{ detail.supplier.phone ?? "—" }}</span>
                  </div>
                  <div class="detail-item">
                    <span class="detail-label">{{ t("suppliers.email") }}</span>
                    <span>{{ detail.supplier.email ?? "—" }}</span>
                  </div>
                </div>
                <div class="col-md-6">
                  <div class="detail-item">
                    <span class="detail-label">{{
                      t("suppliers.address")
                    }}</span>
                    <span>{{ detail.supplier.address ?? "—" }}</span>
                  </div>
                  <div class="detail-item">
                    <span class="detail-label">{{ t("suppliers.taxId") }}</span>
                    <span>{{ detail.supplier.tax_id ?? "—" }}</span>
                  </div>
                  <div class="detail-item">
                    <span class="detail-label">{{
                      t("suppliers.memberSince")
                    }}</span>
                    <span>{{ dateLabel(detail.supplier.created_at) }}</span>
                  </div>
                </div>
              </div>

              <div class="row g-3 mb-3">
                <div class="col">
                  <div class="card text-center p-3">
                    <div class="text-muted small text-uppercase">
                      {{ t("common.invoices") }}
                    </div>
                    <div class="fs-5 fw-semibold">
                      {{ detail.supplier.invoice_count }}
                    </div>
                  </div>
                </div>
                <div class="col">
                  <div class="card text-center p-3">
                    <div class="text-muted small text-uppercase">
                      {{ t("suppliers.totalPurchased") }}
                    </div>
                    <div class="fs-5 fw-semibold">
                      {{ fmt(detail.supplier.total_purchased) }}
                    </div>
                  </div>
                </div>
                <div class="col">
                  <div class="card text-center p-3">
                    <div class="text-muted small text-uppercase">
                      {{ t("suppliers.amountDue") }}
                    </div>
                    <div
                      class="fs-5 fw-semibold"
                      :class="
                        detail.supplier.total_due > 0 ? 'text-danger' : ''
                      "
                    >
                      {{ fmt(detail.supplier.total_due) }}
                    </div>
                  </div>
                </div>
              </div>

              <div class="fw-semibold small text-muted text-uppercase mb-2">
                {{ t("suppliers.recentInvoices") }}
              </div>
              <div v-if="!detail.invoices.length" class="text-muted small py-2">
                {{ t("suppliers.noInvoicesYet") }}
              </div>
              <div v-else class="table-responsive">
                <table class="table table-sm align-middle mb-0">
                  <thead>
                    <tr>
                      <th>{{ t("purchases.invoice") }}</th>
                      <th>{{ t("common.date") }}</th>
                      <th class="text-end">{{ t("common.total") }}</th>
                      <th class="text-end">{{ t("common.paid") }}</th>
                      <th class="text-end">{{ t("common.due") }}</th>
                      <th>{{ t("common.status") }}</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr v-for="inv in detail.invoices" :key="inv.id">
                      <td class="fw-semibold">{{ inv.invoice_no }}</td>
                      <td class="text-muted">{{ inv.date }}</td>
                      <td class="text-end">{{ fmt(inv.total) }}</td>
                      <td class="text-end">{{ fmt(inv.paid_amount) }}</td>
                      <td class="text-end">{{ fmt(inv.due_amount) }}</td>
                      <td>
                        <span class="badge" :class="statusBadge(inv.status)">
                          {{ statusLabel(inv.status) }}
                        </span>
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>
            </template>
          </div>
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
import Paginator from "../../components/Paginator.vue";
import {
  applyFieldRules,
  clearFieldError,
  useFormGuard,
} from "../../composables/useFormGuard";
import { usePagedList, type Paged } from "../../composables/usePagedList";
import { useToast } from "../../composables/useToast";
import { useConfirm } from "../../composables/useConfirm";
import { useSettingsStore } from "../../stores/settings";
import type { Supplier, SupplierDetail } from "../../types";
import { formatMoney } from "../../lib/currency";
import emptySuppliers from "../../assets/empty/suppliers.svg";

const settings = useSettingsStore();
const toast = useToast();
const { confirmDialog } = useConfirm();
const { t, locale } = useI18n();

const search = ref("");

const {
  items: suppliers,
  loading,
  page,
  size,
  totalItems,
  goToPage,
  reload: load,
} = usePagedList<Supplier>(
  (limit, offset) =>
    invoke<Paged<Supplier>>("list_suppliers", {
      search: search.value.trim() || null,
      limit,
      offset,
    }),
  [search],
  (e) => toast.error(e instanceof Error ? e.message : String(e)),
);

const showModal = ref(false);
const saving = ref(false);
const formError = ref("");
const editingId = ref<number | null>(null);
const form = ref({
  name: "",
  contact: "",
  phone: "",
  email: "",
  address: "",
  taxId: "",
});
const errors = reactive<Record<string, string>>({});
const guard = useFormGuard(form);
const canSave = computed(() => guard.isDirty.value && !saving.value);

function resetErrors() {
  for (const key of Object.keys(errors)) delete errors[key];
}

const detail = ref<SupplierDetail | null>(null);
const detailLoading = ref(false);
const detailError = ref("");

function fmt(n: number): string {
  return formatMoney(n ?? 0);
}

function dateLabel(iso: string): string {
  const d = new Date(iso + "Z");
  return Number.isNaN(d.getTime()) ? iso : d.toLocaleDateString(locale.value);
}

function openAdd() {
  formError.value = "";
  editingId.value = null;
  form.value = {
    name: "",
    contact: "",
    phone: "",
    email: "",
    address: "",
    taxId: "",
  };
  resetErrors();
  guard.capture();
  showModal.value = true;
}

function openEdit(s: Supplier) {
  formError.value = "";
  editingId.value = s.id;
  form.value = {
    name: s.name,
    contact: s.contact ?? "",
    phone: s.phone ?? "",
    email: s.email ?? "",
    address: s.address ?? "",
    taxId: s.tax_id ?? "",
  };
  resetErrors();
  guard.capture();
  showModal.value = true;
}

function validate(): boolean {
  const ok = applyFieldRules(errors, [
    ["name", !!form.value.name.trim(), t("common.name")],
  ]);
  if (form.value.email.trim() && !form.value.email.trim().includes("@")) {
    errors.email = t("suppliers.emailInvalid");
  } else if (ok) {
    delete errors.email;
  }
  return ok && !errors.email;
}

function payload() {
  return {
    name: form.value.name,
    contact: form.value.contact || null,
    phone: form.value.phone || null,
    email: form.value.email || null,
    address: form.value.address || null,
    taxId: form.value.taxId || null,
  };
}

async function save() {
  formError.value = "";
  if (!validate()) {
    toast.error(t("common.fixErrors"));
    return;
  }
  saving.value = true;
  try {
    if (editingId.value == null) {
      await invoke<number>("create_supplier", { input: payload() });
      toast.success(t("suppliers.supplierAdded"));
    } else {
      await invoke("update_supplier", {
        supplierId: editingId.value,
        input: payload(),
      });
      toast.success(t("suppliers.supplierUpdated"));
    }
    guard.markSaved();
    showModal.value = false;
    await load();
  } catch (e) {
    formError.value = e instanceof Error ? e.message : String(e);
  } finally {
    saving.value = false;
  }
}

async function openDetail(s: Supplier) {
  detailError.value = "";
  detail.value = null;
  detailLoading.value = true;
  try {
    detail.value = await invoke<SupplierDetail>("get_supplier", {
      supplierId: s.id,
    });
  } catch (e) {
    detailError.value = e instanceof Error ? e.message : String(e);
  } finally {
    detailLoading.value = false;
  }
}

async function remove(s: Supplier) {
  if (
    !(await confirmDialog({
      message: t("suppliers.deleteSupplierConfirm", { name: s.name }),
    }))
  )
    return;
  try {
    await invoke("delete_supplier", { supplierId: s.id });
    toast.success(t("suppliers.supplierDeleted", { name: s.name }));
    await load();
  } catch (e) {
    toast.error(e instanceof Error ? e.message : String(e));
  }
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
  await Promise.allSettled([load(), settings.load()]);
});
</script>

<style scoped>
.detail-item {
  display: flex;
  gap: 0.5rem;
  padding: 0.25rem 0;
  font-size: 0.9rem;
}
.detail-label {
  min-width: 110px;
  color: var(--bs-secondary-color);
}
</style>
