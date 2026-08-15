<template>
  <div>
    <h1 class="h4 mb-3">{{ t("dashboard.title") }}</h1>

    <div class="row g-3 mb-3">
      <div class="col-6 col-lg-3">
        <div class="kpi-card">
          <div class="kpi-icon"><i class="bi bi-cash-stack"></i></div>
          <div>
            <div class="kpi-label">{{ t("dashboard.todayRevenue") }}</div>
            <div class="kpi-value">{{ fmt(kpi.revenue) }}</div>
          </div>
        </div>
      </div>
      <div class="col-6 col-lg-3">
        <div class="kpi-card">
          <div class="kpi-icon"><i class="bi bi-receipt"></i></div>
          <div>
            <div class="kpi-label">{{ t("dashboard.todayOrders") }}</div>
            <div class="kpi-value">{{ kpi.orders }}</div>
          </div>
        </div>
      </div>
      <div class="col-6 col-lg-3">
        <div class="kpi-card">
          <div class="kpi-icon"><i class="bi bi-graph-up-arrow"></i></div>
          <div>
            <div class="kpi-label">{{ t("dashboard.avgTicket") }}</div>
            <div class="kpi-value">{{ fmt(kpi.avgTicket) }}</div>
          </div>
        </div>
      </div>
      <div class="col-6 col-lg-3">
        <div class="kpi-card">
          <div class="kpi-icon"><i class="bi bi-calendar-month"></i></div>
          <div>
            <div class="kpi-label">{{ t("dashboard.thisMonth") }}</div>
            <div class="kpi-value">{{ fmt(kpi.monthRevenue) }}</div>
          </div>
        </div>
      </div>
    </div>

    <div class="row g-3 mb-3">
      <div class="col-6 col-lg-3">
        <div class="kpi-card">
          <div class="kpi-icon"><i class="bi bi-box-seam"></i></div>
          <div>
            <div class="kpi-label">{{ t("dashboard.totalProducts") }}</div>
            <div class="kpi-value">{{ totalProducts }}</div>
          </div>
        </div>
      </div>
      <div class="col-6 col-lg-3">
        <div class="kpi-card">
          <div class="kpi-icon"><i class="bi bi-check-circle"></i></div>
          <div>
            <div class="kpi-label">{{ t("dashboard.activeProducts") }}</div>
            <div class="kpi-value">{{ activeProducts }}</div>
          </div>
        </div>
      </div>
      <div class="col-6 col-lg-3">
        <div class="kpi-card">
          <div class="kpi-icon"><i class="bi bi-boxes"></i></div>
          <div>
            <div class="kpi-label">{{ t("dashboard.unitsOnHand") }}</div>
            <div class="kpi-value">{{ unitsOnHand }}</div>
          </div>
        </div>
      </div>
      <div class="col-6 col-lg-3">
        <div class="kpi-card" :class="lowStock.length ? 'border-danger' : ''">
          <div class="kpi-icon" :class="lowStock.length ? 'text-danger' : 'text-success'">
            <i class="bi bi-exclamation-triangle"></i>
          </div>
          <div>
            <div class="kpi-label">{{ t("dashboard.lowStock") }}</div>
            <div class="kpi-value" :class="lowStock.length ? 'text-danger' : 'text-success'">
              {{ lowStock.length }}
            </div>
          </div>
        </div>
      </div>
    </div>

    <div class="card">
      <div class="card-header d-flex align-items-center justify-content-between">
        <span class="fw-semibold">
          <i class="bi bi-exclamation-triangle me-1 text-warning"></i>{{ t("dashboard.lowStockAlerts") }}
        </span>
        <RouterLink to="/stock" class="btn btn-sm btn-outline-secondary">
          <i class="bi bi-boxes me-1"></i>{{ t("dashboard.goToStock") }}
        </RouterLink>
      </div>
      <div class="table-responsive">
        <table class="table align-middle mb-0">
          <thead>
            <tr>
              <th style="width: 52px"></th>
              <th>{{ t("dashboard.product") }}</th>
              <th>{{ t("dashboard.category") }}</th>
              <th class="text-end">{{ t("dashboard.inStock") }}</th>
              <th class="text-end">{{ t("dashboard.reorderLevel") }}</th>
              <th>{{ t("common.status") }}</th>
            </tr>
          </thead>
          <tbody>
            <tr v-if="!catalog.loaded">
              <td colspan="6" class="text-center text-muted py-4">{{ t("common.loading") }}</td>
            </tr>
            <tr v-else-if="!lowStock.length">
              <td colspan="6" class="text-center text-muted py-4">
                {{ t("dashboard.noLowStock") }}
              </td>
            </tr>
            <tr v-for="p in lowStock" :key="p.id">
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
              <td class="text-end fw-semibold text-danger">{{ p.stock_qty }}</td>
              <td class="text-end text-muted">{{ p.reorder_level }}</td>
              <td>
                <span
                  class="badge"
                  :class="p.stock_qty === 0 ? 'text-bg-danger' : 'text-bg-warning'"
                >
                  {{ p.stock_qty === 0 ? t("dashboard.outOfStock") : t("dashboard.low") }}
                </span>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { RouterLink } from "vue-router";
import { useI18n } from "vue-i18n";
import { useCatalogStore } from "../../stores/catalog";
import { useSettingsStore } from "../../stores/settings";
import { convertFileSrc } from "@tauri-apps/api/core";
import { select } from "../../lib/db";

const catalog = useCatalogStore();
const settings = useSettingsStore();
const { t, locale } = useI18n();

interface SalesKpi {
  revenue: number;
  orders: number;
  avgTicket: number;
  monthRevenue: number;
}

const kpi = ref<SalesKpi>({ revenue: 0, orders: 0, avgTicket: 0, monthRevenue: 0 });

async function loadKpis() {
  const [today, month] = await Promise.all([
    select<{ revenue: number | null; orders: number }>(
      "SELECT COALESCE(SUM(total), 0) AS revenue, COUNT(*) AS orders FROM sales WHERE status = 'completed' AND date(created_at, 'localtime') = date('now', 'localtime')"
    ),
    select<{ revenue: number | null }>(
      "SELECT COALESCE(SUM(total), 0) AS revenue FROM sales WHERE status = 'completed' AND strftime('%Y-%m', created_at, 'localtime') = strftime('%Y-%m', 'now', 'localtime')"
    ),
  ]);
  const t = today[0] ?? { revenue: 0, orders: 0 };
  const revenue = t.revenue ?? 0;
  kpi.value = {
    revenue,
    orders: t.orders,
    avgTicket: t.orders > 0 ? revenue / t.orders : 0,
    monthRevenue: month[0]?.revenue ?? 0,
  };
}

function fmt(n: number): string {
  if (!settings.currency) return n.toFixed(2);
  return new Intl.NumberFormat(locale.value, {
    style: "currency",
    currency: settings.currency,
    currencyDisplay: "narrowSymbol",
  }).format(n);
}

const lowStock = computed(() =>
  catalog.products.filter(
    (p) => p.is_active === 1 && p.reorder_level > 0 && p.stock_qty <= p.reorder_level
  )
);

const totalProducts = computed(() => catalog.products.length);
const activeProducts = computed(
  () => catalog.products.filter((p) => p.is_active === 1).length
);
const unitsOnHand = computed(() =>
  catalog.products.reduce((sum, p) => sum + p.stock_qty, 0)
);

const categoryName = (id: number | null) =>
  catalog.categories.find((c) => c.id === id)?.name ?? "—";

onMounted(async () => {
  await Promise.allSettled([catalog.load(), settings.load(), loadKpis()]);
});
</script>
