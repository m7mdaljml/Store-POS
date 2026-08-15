<template>
  <div>
    <div class="d-flex align-items-center justify-content-between mb-3">
      <h1 class="h4 mb-0">Suppliers</h1>
      <button class="btn btn-primary" type="button" @click="openAdd">
        <i class="bi bi-plus-lg me-1"></i>Add Supplier
      </button>
    </div>

    <div v-if="error" class="alert alert-danger py-2 small" role="alert">
      <i class="bi bi-exclamation-triangle me-1"></i>{{ error }}
    </div>
    <div v-if="notice" class="alert alert-success py-2 small" role="alert">
      <i class="bi bi-check-circle me-1"></i>{{ notice }}
    </div>

    <div class="card">
      <div class="p-2 border-bottom">
        <input
          v-model="search"
          class="form-control form-control-sm"
          type="search"
          placeholder="Search by name, contact, phone, email, address or tax ID…"
        />
      </div>
      <div class="table-responsive">
        <table class="table align-middle mb-0">
          <thead>
            <tr>
              <th>Supplier</th>
              <th>Contact</th>
              <th>Phone</th>
              <th>Tax ID</th>
              <th class="text-end">Invoices</th>
              <th class="text-end">Total purchased</th>
              <th class="text-end">Amount due</th>
              <th class="text-end">Actions</th>
            </tr>
          </thead>
          <tbody>
            <tr v-if="loading">
              <td colspan="8" class="text-center text-muted py-4">Loading…</td>
            </tr>
            <tr v-else-if="!filteredSuppliers.length">
              <td colspan="8" class="text-center text-muted py-4">
                No suppliers yet — click "Add Supplier" to create one
              </td>
            </tr>
            <tr v-for="s in filteredSuppliers" :key="s.id">
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
                  class="btn btn-sm btn-outline-secondary me-1"
                  type="button"
                  title="View details"
                  @click="openDetail(s)"
                >
                  <i class="bi bi-eye"></i>
                </button>
                <button
                  class="btn btn-sm btn-outline-primary me-1"
                  type="button"
                  title="Edit"
                  @click="openEdit(s)"
                >
                  <i class="bi bi-pencil-square"></i>
                </button>
                <button
                  class="btn btn-sm btn-outline-danger"
                  type="button"
                  title="Delete"
                  @click="remove(s)"
                >
                  <i class="bi bi-trash"></i>
                </button>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <div v-if="showModal" class="modal-backdrop show"></div>
    <div v-if="showModal" class="modal d-block" tabindex="-1">
      <div class="modal-dialog modal-dialog-centered">
        <div class="modal-content">
          <form @submit.prevent="save">
            <div class="modal-header">
              <h5 class="modal-title">{{ editingId == null ? "Add Supplier" : "Edit Supplier" }}</h5>
              <button type="button" class="btn-close" @click="showModal = false"></button>
            </div>
            <div class="modal-body">
              <div v-if="formError" class="alert alert-danger py-2 small" role="alert">
                <i class="bi bi-exclamation-triangle me-1"></i>{{ formError }}
              </div>
              <div class="mb-3">
                <label class="form-label" for="s-name">Name *</label>
                <input
                  id="s-name"
                  v-model="form.name"
                  class="form-control"
                  type="text"
                  autofocus
                  required
                />
              </div>
              <div class="mb-3">
                <label class="form-label" for="s-contact">Contact person</label>
                <input id="s-contact" v-model="form.contact" class="form-control" type="text" />
              </div>
              <div class="row g-3">
                <div class="col-md-6">
                  <label class="form-label" for="s-phone">Phone</label>
                  <input id="s-phone" v-model="form.phone" class="form-control" type="text" />
                </div>
                <div class="col-md-6">
                  <label class="form-label" for="s-email">Email</label>
                  <input id="s-email" v-model="form.email" class="form-control" type="email" />
                </div>
              </div>
              <div class="mt-3 mb-3">
                <label class="form-label" for="s-address">Address</label>
                <textarea
                  id="s-address"
                  v-model="form.address"
                  class="form-control"
                  rows="2"
                ></textarea>
              </div>
              <div class="mb-0">
                <label class="form-label" for="s-tax">Tax ID</label>
                <input id="s-tax" v-model="form.taxId" class="form-control" type="text" />
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

    <div v-if="detail || detailLoading" class="modal-backdrop show"></div>
    <div v-if="detail || detailLoading" class="modal d-block" tabindex="-1">
      <div class="modal-dialog modal-lg modal-dialog-centered modal-dialog-scrollable">
        <div class="modal-content">
          <div class="modal-header">
            <h5 class="modal-title">Supplier — {{ detail?.supplier.name }}</h5>
            <button type="button" class="btn-close" @click="detail = null"></button>
          </div>
          <div class="modal-body">
            <div v-if="detailLoading" class="text-center text-muted py-4">Loading…</div>
            <div v-else-if="detailError" class="alert alert-danger py-2 small" role="alert">
              <i class="bi bi-exclamation-triangle me-1"></i>{{ detailError }}
            </div>
            <template v-else-if="detail">
              <div class="row g-3 mb-3">
                <div class="col-md-6">
                  <div class="detail-item">
                    <span class="detail-label">Contact</span>
                    <span>{{ detail.supplier.contact ?? "—" }}</span>
                  </div>
                  <div class="detail-item">
                    <span class="detail-label">Phone</span>
                    <span>{{ detail.supplier.phone ?? "—" }}</span>
                  </div>
                  <div class="detail-item">
                    <span class="detail-label">Email</span>
                    <span>{{ detail.supplier.email ?? "—" }}</span>
                  </div>
                </div>
                <div class="col-md-6">
                  <div class="detail-item">
                    <span class="detail-label">Address</span>
                    <span>{{ detail.supplier.address ?? "—" }}</span>
                  </div>
                  <div class="detail-item">
                    <span class="detail-label">Tax ID</span>
                    <span>{{ detail.supplier.tax_id ?? "—" }}</span>
                  </div>
                  <div class="detail-item">
                    <span class="detail-label">Member since</span>
                    <span>{{ new Date(detail.supplier.created_at + "Z").toLocaleDateString() }}</span>
                  </div>
                </div>
              </div>

              <div class="row g-3 mb-3">
                <div class="col">
                  <div class="card text-center p-3">
                    <div class="text-muted small text-uppercase">Invoices</div>
                    <div class="fs-5 fw-semibold">{{ detail.supplier.invoice_count }}</div>
                  </div>
                </div>
                <div class="col">
                  <div class="card text-center p-3">
                    <div class="text-muted small text-uppercase">Total purchased</div>
                    <div class="fs-5 fw-semibold">{{ fmt(detail.supplier.total_purchased) }}</div>
                  </div>
                </div>
                <div class="col">
                  <div class="card text-center p-3">
                    <div class="text-muted small text-uppercase">Amount due</div>
                    <div
                      class="fs-5 fw-semibold"
                      :class="detail.supplier.total_due > 0 ? 'text-danger' : ''"
                    >
                      {{ fmt(detail.supplier.total_due) }}
                    </div>
                  </div>
                </div>
              </div>

              <div class="fw-semibold small text-muted text-uppercase mb-2">Recent invoices</div>
              <div v-if="!detail.invoices.length" class="text-muted small py-2">
                No invoices yet for this supplier.
              </div>
              <div v-else class="table-responsive">
                <table class="table table-sm align-middle mb-0">
                  <thead>
                    <tr>
                      <th>Invoice</th>
                      <th>Date</th>
                      <th class="text-end">Total</th>
                      <th class="text-end">Paid</th>
                      <th class="text-end">Due</th>
                      <th>Status</th>
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
                          {{ inv.status }}
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
import { computed, onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useSettingsStore } from "../../stores/settings";
import type { Supplier, SupplierDetail } from "../../types";

const settings = useSettingsStore();

const suppliers = ref<Supplier[]>([]);
const search = ref("");
const loading = ref(false);
const error = ref("");
const notice = ref("");

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

const detail = ref<SupplierDetail | null>(null);
const detailLoading = ref(false);
const detailError = ref("");

function fmt(n: number): string {
  if (!settings.currency) return n.toFixed(2);
  return new Intl.NumberFormat(undefined, {
    style: "currency",
    currency: settings.currency,
    currencyDisplay: "narrowSymbol",
  }).format(n);
}

const filteredSuppliers = computed(() => {
  const q = search.value.trim().toLowerCase();
  if (!q) return suppliers.value;
  return suppliers.value.filter((s) =>
    [
      s.name,
      s.contact,
      s.phone,
      s.email,
      s.address,
      s.tax_id,
    ]
      .filter(Boolean)
      .some((v) => (v as string).toLowerCase().includes(q))
  );
});

async function load() {
  loading.value = true;
  error.value = "";
  try {
    suppliers.value = await invoke<Supplier[]>("list_suppliers");
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
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
  showModal.value = true;
}

function validate(): string {
  if (!form.value.name.trim()) return "Supplier name is required";
  if (form.value.email.trim() && !form.value.email.trim().includes("@"))
    return "Enter a valid email address";
  return "";
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
  const err = validate();
  if (err) {
    formError.value = err;
    return;
  }
  saving.value = true;
  try {
    if (editingId.value == null) {
      await invoke<number>("create_supplier", { input: payload() });
      notice.value = "Supplier added";
    } else {
      await invoke("update_supplier", { supplierId: editingId.value, input: payload() });
      notice.value = "Supplier updated";
    }
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
    detail.value = await invoke<SupplierDetail>("get_supplier", { supplierId: s.id });
  } catch (e) {
    detailError.value = e instanceof Error ? e.message : String(e);
  } finally {
    detailLoading.value = false;
  }
}

async function remove(s: Supplier) {
  error.value = "";
  const msg =
    `Delete supplier "${s.name}"?\n\n` +
    "This cannot be undone. Suppliers with invoices on record cannot be deleted.";
  if (!window.confirm(msg)) return;
  try {
    await invoke("delete_supplier", { supplierId: s.id });
    notice.value = `"${s.name}" deleted`;
    await load();
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
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
