<template>
  <div>
    <div class="d-flex align-items-center justify-content-between mb-3">
      <h1 class="h4 mb-0">{{ t("logs.title") }}</h1>
      <div class="d-flex align-items-center gap-2">
        <AppSelect
          v-model="filterUser"
          sm
          class="app-select-inline"
          :items="allUsers"
          :option-label="(u: UserRecord) => u.fullName"
          :option-value="(u: UserRecord) => u.id"
          :placeholder="t('logs.allUsers')"
          style="min-width: 150px"
        />
        <AppSelect
          v-model="filterAction"
          sm
          class="app-select-inline"
          :items="allActions"
          :option-label="(a) => a.label"
          :option-value="(a) => a.value"
          :placeholder="t('logs.allActions')"
          style="min-width: 180px"
        />
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
          :placeholder="t('logs.searchPlaceholder')"
        />
      </div>
      <div class="table-responsive">
        <table class="table table-hover align-middle mb-0">
          <thead v-if="logs.length">
            <tr>
              <th>#</th>
              <th>{{ t("common.date") }}</th>
              <th>{{ t("common.user") }}</th>
              <th>{{ t("logs.action") }}</th>
              <th>{{ t("logs.entity") }}</th>
              <th>{{ t("logs.details") }}</th>
            </tr>
          </thead>
          <tbody v-if="loading">
            <tr>
              <td colspan="6" class="text-center py-4 text-muted">
                <span
                  class="spinner-border spinner-border-sm mx-2"
                  role="status"
                ></span
                >{{ t("common.loading") }}
              </td>
            </tr>
          </tbody>
          <tbody v-else-if="!logs.length">
            <tr>
              <td colspan="6" class="text-center py-4 text-muted">
                {{ t("logs.noLogs") }}
              </td>
            </tr>
          </tbody>
          <tbody v-for="entry in logs" :key="entry.id">
            <tr>
              <td class="text-muted">{{ entry.id }}</td>
              <td class="text-muted">{{ dateLabel(entry.createdAt) }}</td>
              <td>{{ entry.userName || "—" }}</td>
              <td>
                <span class="badge" :class="actionBadgeClass(entry.action)">
                  {{ t("logs.actions." + actionKey(entry.action)) }}
                </span>
              </td>
              <td>
                <span v-if="entry.entityType" class="text-muted">
                  {{ t("logs.entities." + entry.entityType) }}
                  <span v-if="entry.entityId"> #{{ entry.entityId }}</span>
                </span>
                <span v-else>—</span>
              </td>
              <td class="text-muted" style="max-width: 400px">
                {{ translateDetails(entry.action, entry.details) }}
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
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useI18n } from "vue-i18n";
import AppSelect from "../../components/AppSelect.vue";
import Paginator from "../../components/Paginator.vue";
import { usePagedList, type Paged } from "../../composables/usePagedList";
import { useToast } from "../../composables/useToast";
import type { AuditLogEntry } from "../../types";
import type { UserRecord } from "../../types";

const ALL_ACTIONS = [
  "sale.create",
  "sale.hold",
  "sale.resume",
  "sale.cancel",
  "sale.void",
  "sale.refund",
  "session.open",
  "session.close",
  "expense.outgoing",
  "expense.incoming",
  "expense.payment",
  "category.create",
  "category.update",
  "category.delete",
  "product.create",
  "product.update",
  "product.delete",
  "product.activate",
  "product.deactivate",
  "product.import",
  "stock.adjust",
  "supplier.create",
  "supplier.update",
  "supplier.delete",
  "customer.payment",
  "user.create",
  "user.update",
  "user.delete",
  "user.activate",
  "user.deactivate",
  "user.permissions",
];

const toast = useToast();
const { t, locale } = useI18n();

const search = ref("");
const filterUser = ref<number | null>(null);
const filterAction = ref<string | null>(null);
const allUsers = ref<UserRecord[]>([]);

const allActions = computed(() =>
  ALL_ACTIONS.map((a) => ({
    value: a,
    label: t("logs.actions." + a.replace(/\./g, "-")),
  })),
);

const {
  items: logs,
  loading,
  page,
  size,
  totalItems,
  goToPage,
  reload,
} = usePagedList<AuditLogEntry>(
  (limit, offset) =>
    invoke<Paged<AuditLogEntry>>("list_audit_logs", {
      userId: filterUser.value ?? null,
      action: filterAction.value || null,
      search: search.value.trim() || null,
      limit,
      offset,
    }),
  [filterUser, filterAction, search],
  (e) => toast.error(String(e)),
);

function actionKey(action: string): string {
  return action.replace(/\./g, "-");
}

function actionBadgeClass(action: string): string {
  if (action.includes("create")) return "text-bg-success";
  if (action.includes("delete")) return "text-bg-danger";
  if (action.includes("update") || action.includes("edit")) return "text-bg-primary";
  if (action.includes("void") || action.includes("cancel")) return "text-bg-warning";
  if (action.includes("activate")) return "text-bg-success";
  if (action.includes("deactivate")) return "text-bg-secondary";
  if (action.includes("payment")) return "text-bg-info";
  if (action.includes("refund")) return "text-bg-warning";
  return "text-bg-secondary";
}

function dateLabel(d: string): string {
  const date = new Date(d + (d.includes("T") ? "" : "Z"));
  if (isNaN(date.getTime())) return d;
  return date.toLocaleString(locale.value);
}

// Renders a captured money value from an audit detail to two decimals
// ("687.21"), collapsing anything that rounds to zero to "0". This also
// cleans up entries recorded before the backend formatted these amounts.
function cleanMoney(s: string | undefined): string {
  if (!s) return "0";
  const n = parseFloat(s);
  if (Number.isNaN(n)) return s;
  return Math.round(Math.abs(n) * 100) === 0 ? "0" : n.toFixed(2);
}

const detailPatterns: Array<{
  action: string;
  regex: RegExp;
  i18nKey: string;
  map: (m: RegExpMatchArray) => Record<string, string>;
}> = [
  {
    action: "category.create",
    regex: /^Created category "(.+)"$/,
    i18nKey: "logs.detail.createdCategory",
    map: (m) => ({ name: m[1] }),
  },
  {
    action: "category.update",
    regex: /^Updated category "(.+)"$/,
    i18nKey: "logs.detail.updatedCategory",
    map: (m) => ({ name: m[1] }),
  },
  {
    action: "category.delete",
    regex: /^Deleted category "(.+)"$/,
    i18nKey: "logs.detail.deletedCategory",
    map: (m) => ({ name: m[1] }),
  },
  {
    action: "product.create",
    regex: /^Created product "(.+)"$/,
    i18nKey: "logs.detail.createdProduct",
    map: (m) => ({ name: m[1] }),
  },
  {
    action: "product.update",
    regex: /^Updated product "(.+)"$/,
    i18nKey: "logs.detail.updatedProduct",
    map: (m) => ({ name: m[1] }),
  },
  {
    action: "product.delete",
    regex: /^Deleted product "(.+)"$/,
    i18nKey: "logs.detail.deletedProduct",
    map: (m) => ({ name: m[1] }),
  },
  {
    action: "product.activate",
    regex: /^Activated product (\d+)$/,
    i18nKey: "logs.detail.activatedProduct",
    map: (m) => ({ id: m[1] }),
  },
  {
    action: "product.deactivate",
    regex: /^Deactivated product (\d+)$/,
    i18nKey: "logs.detail.deactivatedProduct",
    map: (m) => ({ id: m[1] }),
  },
  {
    action: "product.import",
    regex: /^Imported (\d+) product/,
    i18nKey: "logs.detail.importedProducts",
    map: (m) => ({ count: m[1] }),
  },
  {
    action: "stock.adjust",
    regex: /^Stock adjusted from (\d+(?:\.\d+)?) to (\d+(?:\.\d+)?)$/,
    i18nKey: "logs.detail.stockAdjusted",
    map: (m) => ({ from: m[1], to: m[2] }),
  },
  {
    action: "supplier.create",
    regex: /^Created supplier "(.+)"$/,
    i18nKey: "logs.detail.createdSupplier",
    map: (m) => ({ name: m[1] }),
  },
  {
    action: "supplier.update",
    regex: /^Updated supplier "(.+)"$/,
    i18nKey: "logs.detail.updatedSupplier",
    map: (m) => ({ name: m[1] }),
  },
  {
    action: "supplier.delete",
    regex: /^Deleted supplier "(.+)"$/,
    i18nKey: "logs.detail.deletedSupplier",
    map: (m) => ({ name: m[1] }),
  },
  {
    action: "customer.payment",
    regex: /^Customer payment of (.+)$/,
    i18nKey: "logs.detail.customerPayment",
    map: (m) => ({ amount: cleanMoney(m[1]) }),
  },
  {
    action: "user.create",
    regex: /^Created user "([^"]+)" \((.+)\)$/,
    i18nKey: "logs.detail.createdUser",
    map: (m) => ({ username: m[1], fullName: m[2] }),
  },
  {
    action: "user.update",
    regex: /^Updated user "(.+)"$/,
    i18nKey: "logs.detail.updatedUser",
    map: (m) => ({ username: m[1] }),
  },
  {
    action: "user.delete",
    regex: /^Permanently deleted user (\d+)$/,
    i18nKey: "logs.detail.deleteUser",
    map: (m) => ({ id: m[1] }),
  },
  {
    action: "user.activate",
    regex: /^Reactivated user (\d+)$/,
    i18nKey: "logs.detail.activateUser",
    map: (m) => ({ id: m[1] }),
  },
  {
    action: "user.deactivate",
    regex: /^Deactivated user (\d+)$/,
    i18nKey: "logs.detail.deactivateUser",
    map: (m) => ({ id: m[1] }),
  },
  {
    action: "user.permissions",
    regex: /^Updated permissions for user (\d+): (\d+) permission/,
    i18nKey: "logs.detail.updatedPermissions",
    map: (m) => ({ id: m[1], count: m[2] }),
  },
  {
    action: "sale.create",
    regex: /^Created sale ([^ ]+) for (\d+) item/,
    i18nKey: "logs.detail.createdSale",
    map: (m) => {
      const rest = m.input!.slice(m[0].length);
      const nums = rest.match(
        /total ([\d.,]+), paid ([\d.,]+), change ([\d.,]+)/,
      );
      return {
        no: m[1],
        items: m[2],
        total: cleanMoney(nums?.[1]),
        paid: cleanMoney(nums?.[2]),
        change: cleanMoney(nums?.[3]),
      };
    },
  },
  {
    action: "sale.hold",
    regex: /^Held sale ([^ ]+) for (\d+) item/,
    i18nKey: "logs.detail.heldSale",
    map: (m) => {
      const rest = m.input!.slice(m[0].length);
      const total = rest.match(/total ([\d.,]+)/);
      return { no: m[1], items: m[2], total: cleanMoney(total?.[1]) };
    },
  },
  {
    action: "sale.resume",
    regex: /^Resumed held sale (.+)$/,
    i18nKey: "logs.detail.resumedSale",
    map: (m) => ({ no: m[1] }),
  },
  {
    action: "sale.cancel",
    regex: /^Cancelled held sale (.+)$/,
    i18nKey: "logs.detail.cancelledSale",
    map: (m) => ({ no: m[1] }),
  },
  {
    action: "sale.void",
    regex: /^Voided sale ([^:]+): (.+)$/,
    i18nKey: "logs.detail.voidedSale",
    map: (m) => ({ no: m[1], reason: m[2] }),
  },
  {
    action: "session.open",
    regex: /^Opened register with opening cash (.+)$/,
    i18nKey: "logs.detail.openedRegister",
    map: (m) => ({ cash: cleanMoney(m[1]) }),
  },
  {
    action: "session.close",
    regex: /^Closed register: expected ([\d.,]+), actual ([\d.,]+), variance ([\d.,+-]+)/,
    i18nKey: "logs.detail.closedRegister",
    map: (m) => ({
      expected: cleanMoney(m[1]),
      actual: cleanMoney(m[2]),
      variance: cleanMoney(m[3]),
    }),
  },
  {
    action: "expense.outgoing",
    regex: /^Recorded outgoing expense of (.+)$/,
    i18nKey: "logs.detail.outgoingExpense",
    map: (m) => ({ amount: cleanMoney(m[1]) }),
  },
  {
    action: "expense.incoming",
    regex: /^Created incoming invoice ([^ ]+) for (\d+) line/,
    i18nKey: "logs.detail.incomingInvoice",
    map: (m) => ({ no: m[1], lines: m[2] }),
  },
  {
    action: "expense.payment",
    regex: /^Payment of ([\d.,]+) \((.+)\) on invoice (.+)$/,
    i18nKey: "logs.detail.paymentOnInvoice",
    map: (m) => ({ amount: cleanMoney(m[1]), method: m[2], no: m[3] }),
  },
];

function translateDetails(action: string, details: string | null): string {
  if (!details) return "—";
  const pattern = detailPatterns.find((p) => p.action === action);
  if (!pattern) return details;
  const match = details.match(pattern.regex);
  if (!match) return details;
  return t(pattern.i18nKey, pattern.map(match));
}

onMounted(async () => {
  try {
    const p = await invoke<Paged<UserRecord>>("list_users", {});
    allUsers.value = p.items;
  } catch (e) {
    toast.error(String(e));
  }
  await reload();
});
</script>
