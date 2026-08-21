<template>
  <div class="page-container">
    <div class="d-flex align-items-center justify-content-between mb-3">
      <h1 class="h4 mb-0">{{ t("customers.title") }}</h1>
      <button class="btn btn-primary" type="button" @click="openAdd">
        <i class="bi bi-person-plus me-1"></i>{{ t("customers.addCustomer") }}
      </button>
    </div>

    <div v-if="error" class="alert alert-danger py-2 small" role="alert">
      <i class="bi bi-exclamation-triangle me-1"></i>{{ error }}
    </div>
    <div v-if="notice" class="alert alert-success py-2 small" role="alert">
      <i class="bi bi-check-circle me-1"></i>{{ notice }}
    </div>

    <div class="row g-3 mb-3">
      <div class="col-md-4">
        <div class="kpi-card">
          <div class="kpi-icon"><i class="bi bi-people"></i></div>
          <div>
            <div class="kpi-label">{{ t("customers.totalCustomers") }}</div>
            <div class="kpi-value">{{ customers.length }}</div>
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
          <thead>
            <tr>
              <th>{{ t("common.name") }}</th>
              <th>{{ t("customers.phone") }}</th>
              <th>{{ t("suppliers.email") }}</th>
              <th>{{ t("suppliers.address") }}</th>
              <th class="text-end">{{ t("customers.balance") }}</th>
              <th class="text-end">{{ t("customers.memberSince") }}</th>
              <th class="text-end">{{ t("common.actions") }}</th>
            </tr>
          </thead>
          <tbody>
            <tr v-if="loading">
              <td colspan="7" class="text-center text-muted py-4">{{ t("common.loading") }}</td>
            </tr>
            <tr v-else-if="!filteredCustomers.length">
              <td colspan="7" class="text-center text-muted py-4">
                {{ search ? t("customers.noMatching") : t("customers.noCustomers") }}
              </td>
            </tr>
            <tr v-for="c in filteredCustomers" :key="c.id">
              <td>
                <div class="fw-semibold">{{ c.name }}</div>
                <div v-if="c.notes" class="text-muted text-xs">{{ c.notes }}</div>
              </td>
              <td class="text-muted" style="direction: ltr">{{ c.phone ?? "—" }}</td>
              <td class="text-muted">{{ c.email ?? "—" }}</td>
              <td class="text-muted">{{ c.address ?? "—" }}</td>
              <td class="text-end">
                <span v-if="c.balance > 0.005" class="badge text-bg-danger rounded-pill">
                  {{ fmt(c.balance) }}
                </span>
                <span v-else class="text-muted">—</span>
              </td>
              <td class="text-end text-muted">{{ dateLabel(c.created_at) }}</td>
              <td class="text-end text-nowrap">
                <button
                  class="btn btn-sm btn-outline-primary"
                  type="button"
                  :title="t('common.edit')"
                  @click="openEdit(c)"
                >
                  <i class="bi bi-pencil-square"></i>
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
          <form novalidate @submit.prevent="save">
            <div class="modal-header">
              <h5 class="modal-title">
                {{ editingId == null ? t("customers.addTitle") : t("customers.editTitle", { name: form.name }) }}
              </h5>
              <button type="button" class="btn-close" @click="showModal = false"></button>
            </div>
            <div class="modal-body">
              <div class="mb-3">
                <label class="form-label" for="c-name">{{ t("customers.nameRequired") }}</label>
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
                  <label class="form-label" for="c-phone">{{ t("customers.phone") }} {{ t("common.optional") }}</label>
                  <input
                    id="c-phone"
                    v-model="form.phone"
                    class="form-control"
                    :class="{ 'is-invalid': !!fieldErrors.phone }"
                    type="tel"
                    style="direction: ltr"
                  />
                  <div v-if="fieldErrors.phone" class="invalid-feedback d-block">
                    {{ fieldErrors.phone }}
                  </div>
                </div>
                <div class="col-md-6">
                  <label class="form-label" for="c-email">{{ t("suppliers.email") }} {{ t("common.optional") }}</label>
                  <input
                    id="c-email"
                    v-model="form.email"
                    class="form-control"
                    :class="{ 'is-invalid': !!fieldErrors.email }"
                    type="email"
                  />
                  <div v-if="fieldErrors.email" class="invalid-feedback d-block">
                    {{ fieldErrors.email }}
                  </div>
                </div>
              </div>
              <div class="mb-3">
                <label class="form-label" for="c-address">{{ t("suppliers.address") }} {{ t("common.optional") }}</label>
                <textarea id="c-address" v-model="form.address" class="form-control" rows="2"></textarea>
              </div>
              <div class="mb-0">
                <label class="form-label" for="c-notes">{{ t("common.notes") }} {{ t("common.optional") }}</label>
                <textarea id="c-notes" v-model="form.notes" class="form-control" rows="2"></textarea>
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
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useSettingsStore } from "../../stores/settings";
import { execute, insert, select } from "../../lib/db";
import type { Customer } from "../../types";

const settings = useSettingsStore();
const { t, locale } = useI18n();

const customers = ref<Customer[]>([]);
const search = ref("");
const loading = ref(false);
const error = ref("");
const notice = ref("");

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

const filteredCustomers = computed(() => {
  const q = search.value.trim().toLowerCase();
  if (!q) return customers.value;
  return customers.value.filter((c) =>
    [c.name, c.phone, c.email, c.address, c.notes]
      .filter(Boolean)
      .some((v) => (v as string).toLowerCase().includes(q))
  );
});

const debtorsCount = computed(
  () => customers.value.filter((c) => c.balance > 0.005).length
);

const totalDebt = computed(() =>
  customers.value.reduce((sum, c) => sum + Math.max(c.balance, 0), 0)
);

function fmt(n: number): string {
  if (!settings.currency) return n.toFixed(2);
  return new Intl.NumberFormat(locale.value, {
    style: "currency",
    currency: settings.currency,
    currencyDisplay: "narrowSymbol",
  }).format(n);
}

function dateLabel(iso: string): string {
  const d = new Date(iso + (iso.includes("T") ? "" : "Z"));
  return Number.isNaN(d.getTime())
    ? iso
    : d.toLocaleDateString(locale.value);
}

async function load() {
  loading.value = true;
  error.value = "";
  try {
    customers.value = await select<Customer>(
      "SELECT id, name, phone, email, address, balance, notes, created_at FROM customers ORDER BY name COLLATE NOCASE"
    );
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

function openAdd() {
  editingId.value = null;
  fieldErrors.value = {};
  form.value = { name: "", phone: "", email: "", address: "", notes: "" };
  showModal.value = true;
}

function openEdit(c: Customer) {
  error.value = "";
  notice.value = "";
  editingId.value = c.id;
  fieldErrors.value = {};
  form.value = {
    name: c.name,
    phone: c.phone ?? "",
    email: c.email ?? "",
    address: c.address ?? "",
    notes: c.notes ?? "",
  };
  showModal.value = true;
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
  if (!validate()) return;
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
        ]
      );
      notice.value = t("customers.customerAdded", { name: form.value.name.trim() });
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
        ]
      );
      notice.value = t("customers.customerUpdated", { name: form.value.name.trim() });
    }
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
