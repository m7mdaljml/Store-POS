<template>
  <div>
    <div class="d-flex align-items-center justify-content-between mb-3">
      <h1 class="h4 mb-0">{{ t("reports.title") }}</h1>
      <div class="d-flex flex-wrap align-items-center gap-2">
        <div
          class="btn-group btn-group-sm"
          role="group"
          :aria-label="t('reports.period')"
        >
          <button
            v-for="p in presets"
            :key="p.key"
            class="btn"
            :class="preset === p.key ? 'btn-primary' : 'btn-outline-secondary'"
            type="button"
            @click="setPreset(p.key)"
          >
            {{ t(p.label) }}
          </button>
        </div>
        <template v-if="preset === 'custom'">
          <input
            v-model="from"
            class="form-control form-control-sm"
            type="date"
            :aria-label="t('reports.fromDate')"
          />
          <input
            v-model="to"
            class="form-control form-control-sm"
            type="date"
            :aria-label="t('reports.toDate')"
          />
        </template>
      </div>
    </div>

    <ul class="nav nav-pills pos-tabs mb-3">
      <li v-for="tb in tabs" :key="tb" class="nav-item">
        <button
          class="nav-link"
          :class="{ active: activeTab === tb }"
          type="button"
          @click="activeTab = tb"
        >
          {{ t(`reports.${tb}Tab`) }}
        </button>
      </li>
    </ul>

    <div v-if="loading && !anyLoaded" class="text-center text-muted py-5">
      <span class="spinner-border mx-2"></span>{{ t("common.loading") }}
    </div>

    <!-- ============ F6.2 Overview ============ -->
    <template v-if="activeTab === 'overview'">
      <div v-if="summary" class="row g-3 mb-3">
        <div class="col-6 col-md-4 col-xl-2">
          <div class="kpi-card">
            <div class="kpi-icon"><i class="bi bi-cash-stack"></i></div>
            <div>
              <div class="kpi-label">{{ t("reports.kpiRevenue") }}</div>
              <div class="kpi-value">{{ fmt(summary.revenue) }}</div>
            </div>
          </div>
        </div>
        <div class="col-6 col-md-4 col-xl-2">
          <div class="kpi-card">
            <div class="kpi-icon"><i class="bi bi-receipt"></i></div>
            <div>
              <div class="kpi-label">{{ t("reports.kpiOrders") }}</div>
              <div class="kpi-value">{{ summary.orders }}</div>
            </div>
          </div>
        </div>
        <div class="col-6 col-md-4 col-xl-2">
          <div class="kpi-card">
            <div class="kpi-icon"><i class="bi bi-basket"></i></div>
            <div>
              <div class="kpi-label">{{ t("reports.kpiAvgTicket") }}</div>
              <div class="kpi-value">{{ fmt(summary.avgTicket) }}</div>
            </div>
          </div>
        </div>
        <div class="col-6 col-md-4 col-xl-2">
          <div class="kpi-card">
            <div class="kpi-icon"><i class="bi bi-graph-up-arrow"></i></div>
            <div>
              <div class="kpi-label">{{ t("reports.kpiGrossProfit") }}</div>
              <div class="kpi-value">{{ fmt(summary.grossProfit) }}</div>
            </div>
          </div>
        </div>
        <div class="col-6 col-md-4 col-xl-2">
          <div class="kpi-card">
            <div class="kpi-icon"><i class="bi bi-wallet2"></i></div>
            <div>
              <div class="kpi-label">{{ t("reports.kpiExpenses") }}</div>
              <div class="kpi-value text-danger">
                {{ fmt(summary.expensesTotal) }}
              </div>
            </div>
          </div>
        </div>
        <div class="col-6 col-md-4 col-xl-2">
          <div class="kpi-card">
            <div class="kpi-icon"><i class="bi bi-speedometer2"></i></div>
            <div>
              <div class="kpi-label">{{ t("reports.kpiNet") }}</div>
              <div
                class="kpi-value"
                :class="
                  summary.netPosition >= 0 ? 'text-success' : 'text-danger'
                "
              >
                {{ fmt(summary.netPosition) }}
              </div>
            </div>
          </div>
        </div>
      </div>

      <div class="row g-3 mb-3">
        <div class="col-lg-8">
          <div class="card h-100">
            <div
              class="card-header d-flex align-items-center justify-content-between py-2"
            >
              <span class="fw-semibold small text-uppercase text-muted">{{
                t("reports.trendTitle")
              }}</span>
              <div class="btn-group btn-group-sm" role="group">
                <button
                  v-for="g in granularities"
                  :key="g.key"
                  class="btn"
                  :class="
                    granularity === g.key
                      ? 'btn-primary'
                      : 'btn-outline-secondary'
                  "
                  type="button"
                  @click="granularity = g.key"
                >
                  {{ t(g.label) }}
                </button>
              </div>
            </div>
            <div class="card-body chart-box">
              <EmptyState
                v-if="!trendPoints.length"
                :image="emptySales"
                :message="t('reports.noData')"
              />
              <canvas v-else ref="trendCanvas"></canvas>
            </div>
          </div>
        </div>
        <div class="col-lg-4">
          <div class="card h-100">
            <div class="card-header py-2">
              <span class="fw-semibold small text-uppercase text-muted">{{
                t("reports.catBreakdown")
              }}</span>
            </div>
            <div class="card-body chart-box">
              <EmptyState
                v-if="!categories.length"
                :image="emptySales"
                :message="t('reports.noData')"
              />
              <canvas v-else ref="catCanvas"></canvas>
            </div>
          </div>
        </div>
      </div>

      <div class="card">
        <div
          class="card-header d-flex align-items-center justify-content-between py-2"
        >
          <span class="fw-semibold small text-uppercase text-muted">{{
            t("reports.topProductsMini")
          }}</span>
          <button
            class="btn btn-sm btn-outline-secondary"
            type="button"
            @click="activeTab = 'products'"
          >
            {{ t("reports.viewAll") }} <i class="bi bi-arrow-right ms-1"></i>
          </button>
        </div>
        <div class="table-responsive">
          <table class="table table-sm align-middle mb-0">
            <thead v-if="topProducts.length">
              <tr>
                <th style="width: 40px">#</th>
                <th>{{ t("common.name") }}</th>
                <th>{{ t("reports.category") }}</th>
                <th class="text-end">{{ t("reports.qtySold") }}</th>
                <th class="text-end">{{ t("reports.revenue") }}</th>
                <th class="text-end">{{ t("reports.profit") }}</th>
              </tr>
            </thead>
            <tbody v-if="!topProducts.length">
              <tr>
                <td colspan="6" class="p-0 border-0">
                  <EmptyState
                    :image="emptySales"
                    :message="t('reports.noData')"
                  />
                </td>
              </tr>
            </tbody>
            <tbody v-for="(p, i) in topProducts.slice(0, 5)" :key="p.productId">
              <tr>
                <td class="fw-semibold">{{ i + 1 }}</td>
                <td class="fw-semibold">{{ p.name }}</td>
                <td class="text-muted">{{ p.category ?? "—" }}</td>
                <td class="text-end">{{ num(p.qty) }}</td>
                <td class="text-end">{{ fmt(p.revenue) }}</td>
                <td
                  class="text-end"
                  :class="p.profit >= 0 ? 'text-success' : 'text-danger'"
                >
                  {{ fmt(p.profit) }}
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </template>

    <!-- ============ F6.3 Sales report ============ -->
    <template v-if="activeTab === 'sales'">
      <div class="card mb-3">
        <div class="card-body py-2">
          <div class="row g-2 align-items-center">
            <div class="col-auto">
              <select
                v-model.number="salesFilters.cashierId"
                class="form-select form-select-sm"
              >
                <option :value="null">{{ t("reports.cashierAll") }}</option>
                <option v-for="u in cashiers" :key="u.id" :value="u.id">
                  {{ u.fullName }}
                </option>
              </select>
            </div>
            <div class="col-auto">
              <select
                v-model.number="salesFilters.customerId"
                class="form-select form-select-sm"
              >
                <option :value="null">{{ t("reports.customerAll") }}</option>
                <option v-for="c in customerOptions" :key="c.id" :value="c.id">
                  {{ customerLabel(c) }}
                </option>
              </select>
            </div>
            <div class="col-auto">
              <div class="form-check mb-0">
                <input
                  id="inc-voided"
                  v-model="salesFilters.includeVoided"
                  class="form-check-input"
                  type="checkbox"
                />
                <label class="form-check-label small" for="inc-voided">{{
                  t("reports.includeVoided")
                }}</label>
              </div>
            </div>
            <div class="col-auto ms-auto" v-can="'export.excel'">
              <AsyncButton
                size="sm"
                variant="outline-primary"
                :loading="exporting"
                @click="exportSales"
              >
                <i v-if="!exporting" class="bi bi-file-earmark-excel mx-1"></i
                >{{ t("common.export") }}
              </AsyncButton>
            </div>
          </div>
        </div>
      </div>

      <div v-if="salesData" class="row g-3 mb-3">
        <div class="col-md-4">
          <div class="kpi-card">
            <div class="kpi-icon"><i class="bi bi-receipt"></i></div>
            <div>
              <div class="kpi-label">{{ t("reports.kpiOrders") }}</div>
              <div class="kpi-value">{{ salesData.orders }}</div>
            </div>
          </div>
        </div>
        <div class="col-md-4">
          <div class="kpi-card">
            <div class="kpi-icon"><i class="bi bi-cash-stack"></i></div>
            <div>
              <div class="kpi-label">{{ t("reports.kpiRevenue") }}</div>
              <div class="kpi-value">{{ fmt(salesData.revenue) }}</div>
            </div>
          </div>
        </div>
        <div class="col-md-4">
          <div class="kpi-card">
            <div class="kpi-icon"><i class="bi bi-basket"></i></div>
            <div>
              <div class="kpi-label">{{ t("reports.kpiAvgTicket") }}</div>
              <div class="kpi-value">{{ fmt(salesData.avgTicket) }}</div>
            </div>
          </div>
        </div>
      </div>

      <div class="card">
        <div class="table-responsive">
          <table class="table table-sm align-middle mb-0">
            <thead v-if="salesRows.length">
              <tr>
                <th>{{ t("sales.saleNo") }}</th>
                <th>{{ t("common.date") }}</th>
                <th>{{ t("common.cashier") }}</th>
                <th>{{ t("common.customer") }}</th>
                <th class="text-end">{{ t("reports.subtotal") }}</th>
                <th class="text-end">{{ t("reports.discount") }}</th>
                <th class="text-end">{{ t("common.total") }}</th>
                <th>{{ t("common.status") }}</th>
              </tr>
            </thead>
            <tbody v-if="!salesRows.length">
              <tr>
                <td colspan="8" class="p-0 border-0">
                  <EmptyState
                    :image="emptySales"
                    :message="t('reports.noData')"
                  />
                </td>
              </tr>
            </tbody>
            <tbody v-for="r in salesRows" :key="r.id">
              <tr>
                <td class="fw-semibold">{{ r.saleNo }}</td>
                <td class="text-muted">{{ dateLabel(r.createdAt) }}</td>
                <td>{{ r.cashier }}</td>
                <td>{{ r.customer ?? "—" }}</td>
                <td class="text-end">{{ fmt(r.subtotal) }}</td>
                <td class="text-end text-danger">
                  {{ r.discount ? `−${fmt(r.discount)}` : "—" }}
                </td>
                <td
                  class="text-end fw-semibold"
                  :title="r.refunded ? `${fmt(r.total)} − ${fmt(r.refunded)}` : undefined"
                >
                  {{ fmt(r.total - Math.min(r.refunded, r.total)) }}
                </td>
                <td>
                  <span
                    class="badge"
                    :class="
                      r.status === 'completed'
                        ? 'text-bg-success'
                        : 'text-bg-secondary'
                    "
                  >
                    {{
                      r.status === "completed"
                        ? t("customers.statusCompleted")
                        : t("customers.statusVoided")
                    }}
                  </span>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </template>

    <!-- ============ F6.4 Top products ============ -->
    <template v-if="activeTab === 'products'">
      <div class="card">
        <div
          class="card-header d-flex align-items-center justify-content-between py-2"
        >
          <span class="fw-semibold small text-uppercase text-muted">{{
            t("reports.topProductsMini")
          }}</span>
          <AsyncButton
            v-can="'export.excel'"
            size="sm"
            variant="outline-primary"
            :loading="exporting"
            @click="exportTopProducts"
          >
            <i v-if="!exporting" class="bi bi-file-earmark-excel mx-1"></i
            >{{ t("common.export") }}
          </AsyncButton>
        </div>
        <div class="table-responsive">
          <table class="table table-sm align-middle mb-0">
            <thead v-if="topProducts.length">
              <tr>
                <th style="width: 40px">{{ t("reports.rank") }}</th>
                <th>{{ t("common.name") }}</th>
                <th>{{ t("reports.category") }}</th>
                <th class="text-end">{{ t("reports.qtySold") }}</th>
                <th class="text-end">{{ t("reports.revenue") }}</th>
                <th class="text-end">{{ t("reports.profit") }}</th>
              </tr>
            </thead>
            <tbody v-if="!topProducts.length">
              <tr>
                <td colspan="6" class="p-0 border-0">
                  <EmptyState
                    :image="emptySales"
                    :message="t('reports.noData')"
                  />
                </td>
              </tr>
            </tbody>
            <tbody v-for="(p, i) in topProducts" :key="p.productId">
              <tr>
                <td class="fw-semibold">{{ i + 1 }}</td>
                <td class="fw-semibold">{{ p.name }}</td>
                <td class="text-muted">{{ p.category ?? "—" }}</td>
                <td class="text-end">{{ num(p.qty) }}</td>
                <td class="text-end">{{ fmt(p.revenue) }}</td>
                <td
                  class="text-end"
                  :class="p.profit >= 0 ? 'text-success' : 'text-danger'"
                >
                  {{ fmt(p.profit) }}
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </template>

    <!-- ============ F6.6 Inventory ============ -->
    <template v-if="activeTab === 'inventory'">
      <div v-if="inventory.length" class="row g-3 mb-3">
        <div class="col-md-4">
          <div class="kpi-card">
            <div class="kpi-icon"><i class="bi bi-boxes"></i></div>
            <div>
              <div class="kpi-label">{{ t("reports.invProductCount") }}</div>
              <div class="kpi-value">{{ inventory.length }}</div>
            </div>
          </div>
        </div>
        <div class="col-md-4">
          <div class="kpi-card">
            <div class="kpi-icon"><i class="bi bi-safe2"></i></div>
            <div>
              <div class="kpi-label">{{ t("reports.invStockValue") }}</div>
              <div class="kpi-value">{{ fmt(totalStockValue) }}</div>
            </div>
          </div>
        </div>
        <div class="col-md-4">
          <div class="kpi-card">
            <div class="kpi-icon">
              <i class="bi bi-exclamation-triangle"></i>
            </div>
            <div>
              <div class="kpi-label">{{ t("reports.lowStockCount") }}</div>
              <div
                class="kpi-value"
                :class="lowStockItems.length ? 'text-warning' : ''"
              >
                {{ lowStockItems.length }}
              </div>
            </div>
          </div>
        </div>
      </div>

      <div
        v-if="lowStockItems.length"
        class="alert alert-warning py-2 small d-flex align-items-center gap-2"
        role="alert"
      >
        <i class="bi bi-exclamation-triangle-fill"></i>
        {{ t("reports.lowStockAlert", { count: lowStockItems.length }) }}
        <button
          class="btn btn-sm btn-outline-dark ms-auto"
          type="button"
          @click="invLowOnly = true"
        >
          {{ t("reports.lowStockOnly") }}
        </button>
      </div>

      <div class="card">
        <div
          class="card-body d-flex gap-2 py-2 flex-wrap"
          style="
            border-bottom: var(--bs-card-border-width) solid
              var(--bs-card-border-color);
          "
        >
          <input
            v-model="invSearch"
            class="form-control form-control-sm w-auto flex-grow-1"
            type="search"
            :placeholder="t('reports.searchInventory')"
          />
          <div class="form-check mb-0 align-self-center">
            <input
              id="low-only"
              v-model="invLowOnly"
              class="form-check-input"
              type="checkbox"
            />
            <label class="form-check-label small" for="low-only">{{
              t("reports.lowStockOnly")
            }}</label>
          </div>
          <button
            class="btn btn-sm btn-outline-secondary"
            type="button"
            @click="showMovements = true"
          >
            <i class="bi bi-arrow-left-right mx-1"></i
            >{{ t("reports.movementsTitle") }}
          </button>
          <AsyncButton
            v-can="'export.excel'"
            size="sm"
            variant="outline-primary"
            :loading="exporting"
            @click="exportInventory"
          >
            <i v-if="!exporting" class="bi bi-file-earmark-excel mx-1"></i
            >{{ t("common.export") }}
          </AsyncButton>
        </div>
        <div class="table-responsive">
          <table class="table table-sm align-middle mb-0">
            <thead v-if="visibleInventory.length">
              <tr>
                <th>{{ t("common.name") }}</th>
                <th>{{ t("reports.category") }}</th>
                <th class="text-end">{{ t("reports.stockCol") }}</th>
                <th class="text-end">{{ t("reports.reorderCol") }}</th>
                <th class="text-end">{{ t("reports.costPrice") }}</th>
                <th class="text-end">{{ t("reports.sellPrice") }}</th>
                <th class="text-end">{{ t("reports.stockValueCol") }}</th>
              </tr>
            </thead>
            <tbody v-if="!visibleInventory.length">
              <tr>
                <td colspan="8" class="p-0 border-0">
                  <EmptyState
                    :image="emptyProducts"
                    :message="t('reports.noData')"
                  />
                </td>
              </tr>
            </tbody>
            <tbody v-for="i in visibleInventory" :key="i.id">
              <tr>
                <td class="fw-semibold">
                  {{ i.name }}
                  <span v-if="i.lowStock" class="badge text-bg-warning ms-1">{{
                    t("reports.low")
                  }}</span>
                </td>
                <td class="text-muted">{{ i.category ?? "—" }}</td>
                <td
                  class="text-end fw-semibold"
                  :class="{ 'text-warning': i.lowStock }"
                >
                  {{ num(i.stockQty) }}
                </td>
                <td class="text-end text-muted">{{ num(i.reorderLevel) }}</td>
                <td class="text-end">{{ fmt(i.costPrice) }}</td>
                <td class="text-end">{{ fmt(i.sellPrice) }}</td>
                <td class="text-end fw-semibold">{{ fmt(i.stockValue) }}</td>
              </tr>
            </tbody>
          </table>
        </div>
        <div
          v-if="visibleInventory.length < filteredInventory.length"
          class="text-center border-top py-2"
        >
          <button
            class="btn btn-sm btn-outline-secondary"
            type="button"
            @click="showMoreInventory"
          >
            {{ t("common.loadMore") }}
            <span class="text-muted small mx-1">
              ({{ visibleInventory.length }}/{{ filteredInventory.length }})
            </span>
          </button>
        </div>
      </div>
    </template>

    <!-- ============ F6.7 Margins ============ -->
    <template v-if="activeTab === 'margins'">
      <div class="card mb-3">
        <div class="card-header py-2">
          <span class="fw-semibold small text-uppercase text-muted">{{
            t("reports.marginByCategory")
          }}</span>
        </div>
        <div class="table-responsive">
          <table class="table table-sm align-middle mb-0">
            <thead v-if="marginData?.categories.length">
              <tr>
                <th>{{ t("reports.category") }}</th>
                <th class="text-end">{{ t("reports.qtySold") }}</th>
                <th class="text-end">{{ t("reports.revenue") }}</th>
                <th class="text-end">{{ t("reports.cogs") }}</th>
                <th class="text-end">{{ t("reports.profit") }}</th>
                <th class="text-end">{{ t("reports.marginPct") }}</th>
              </tr>
            </thead>
            <tbody v-if="!marginData?.categories.length">
              <tr>
                <td colspan="6" class="p-0 border-0">
                  <EmptyState
                    :image="emptyExpenses"
                    :message="t('reports.noData')"
                  />
                </td>
              </tr>
            </tbody>
            <tbody v-for="c in marginData?.categories ?? []" :key="c.name">
              <tr>
                <td class="fw-semibold">{{ c.name }}</td>
                <td class="text-end">{{ num(c.qtySold) }}</td>
                <td class="text-end">{{ fmt(c.revenue) }}</td>
                <td class="text-end text-muted">{{ fmt(c.cogs) }}</td>
                <td
                  class="text-end"
                  :class="c.profit >= 0 ? 'text-success' : 'text-danger'"
                >
                  {{ fmt(c.profit) }}
                </td>
                <td class="text-end">
                  <span
                    class="badge rounded-pill"
                    :class="
                      c.marginPct >= 20
                        ? 'text-bg-success'
                        : c.marginPct > 0
                          ? 'text-bg-warning'
                          : 'text-bg-danger'
                    "
                  >
                    {{ c.marginPct.toFixed(1) }}%
                  </span>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      <div class="card">
        <div class="card-header py-2">
          <span class="fw-semibold small text-uppercase text-muted">{{
            t("reports.marginByProduct")
          }}</span>
        </div>
        <div class="table-responsive">
          <table class="table table-sm align-middle mb-0">
            <thead v-if="marginData?.products.length">
              <tr>
                <th>{{ t("common.name") }}</th>
                <th>{{ t("reports.category") }}</th>
                <th class="text-end">{{ t("reports.qtySold") }}</th>
                <th class="text-end">{{ t("reports.revenue") }}</th>
                <th class="text-end">{{ t("reports.cogs") }}</th>
                <th class="text-end">{{ t("reports.profit") }}</th>
                <th class="text-end">{{ t("reports.marginPct") }}</th>
              </tr>
            </thead>
            <tbody v-if="!marginData?.products.length">
              <tr>
                <td colspan="7" class="p-0 border-0">
                  <EmptyState
                    :image="emptyProducts"
                    :message="t('reports.noData')"
                  />
                </td>
              </tr>
            </tbody>
            <tbody v-for="p in marginData?.products ?? []" :key="p.id">
              <tr>
                <td class="fw-semibold">{{ p.name }}</td>
                <td class="text-muted">{{ p.category ?? "—" }}</td>
                <td class="text-end">{{ num(p.qtySold) }}</td>
                <td class="text-end">{{ fmt(p.revenue) }}</td>
                <td class="text-end text-muted">{{ fmt(p.cogs) }}</td>
                <td
                  class="text-end"
                  :class="p.profit >= 0 ? 'text-success' : 'text-danger'"
                >
                  {{ fmt(p.profit) }}
                </td>
                <td class="text-end">
                  <span
                    class="badge rounded-pill"
                    :class="
                      p.marginPct >= 20
                        ? 'text-bg-success'
                        : p.marginPct > 0
                          ? 'text-bg-warning'
                          : 'text-bg-danger'
                    "
                  >
                    {{ p.marginPct.toFixed(1) }}%
                  </span>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </template>

    <!-- Stock movements modal (F6.6) -->
    <div v-if="showMovements" class="modal-backdrop show"></div>
    <div v-if="showMovements" class="modal d-block" tabindex="-1">
      <div
        class="modal-dialog modal-lg modal-dialog-centered modal-dialog-scrollable"
      >
        <div class="modal-content">
          <div class="modal-header">
            <h5 class="modal-title">{{ t("reports.movementsTitle") }}</h5>
            <button
              type="button"
              class="btn-close"
              @click="showMovements = false"
            ></button>
          </div>
          <div class="modal-body">
            <div v-if="movementsLoading" class="text-center text-muted py-4">
              {{ t("common.loading") }}
            </div>
            <EmptyState
              v-else-if="!movements.length"
              :image="emptyMovements"
              :message="t('reports.noData')"
            />
            <table v-else class="table table-sm align-middle mb-0">
              <thead>
                <tr>
                  <th>{{ t("common.date") }}</th>
                  <th>{{ t("common.product") }}</th>
                  <th>{{ t("stock.type") }}</th>
                  <th class="text-end">{{ t("reports.movementsQty") }}</th>
                  <th>{{ t("common.cashier") }}</th>
                  <th>{{ t("common.notes") }}</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="m in movements" :key="m.id">
                  <td class="text-muted">{{ dateLabel(m.createdAt) }}</td>
                  <td class="fw-semibold">{{ m.productName }}</td>
                  <td>
                    <span class="badge text-bg-light border">{{
                      m.movementType
                    }}</span>
                  </td>
                  <td
                    class="text-end"
                    :class="m.qty >= 0 ? 'text-success' : 'text-danger'"
                  >
                    {{ m.qty >= 0 ? "+" : "" }}{{ num(m.qty) }}
                  </td>
                  <td class="text-muted">{{ m.userName ?? "—" }}</td>
                  <td class="text-muted">{{ m.notes ?? "—" }}</td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import {
  computed,
  nextTick,
  onBeforeUnmount,
  onMounted,
  ref,
  watch,
} from "vue";
import { invoke } from "@tauri-apps/api/core";
import { save as saveDialog } from "@tauri-apps/plugin-dialog";
import Chart from "chart.js/auto";
import { useI18n } from "vue-i18n";
import EmptyState from "../../components/EmptyState.vue";
import AsyncButton from "../../components/AsyncButton.vue";
import { useSettingsStore } from "../../stores/settings";
import { useToast } from "../../composables/useToast";
import { select } from "../../lib/db";
import { formatMoney } from "../../lib/currency";
import emptySales from "../../assets/empty/sales.svg";
import emptyProducts from "../../assets/empty/products.svg";
import emptyExpenses from "../../assets/empty/expenses.svg";
import emptyMovements from "../../assets/empty/movements.svg";
import type {
  CategorySalesRow,
  CustomerLite,
  InventoryRow,
  MarginReport,
  SalesReportOutput,
  SalesSummary,
  StockMovement,
  TopProductRow,
  TrendPoint,
  UserRecord,
} from "../../types";

const settings = useSettingsStore();
const toast = useToast();
const { t } = useI18n();

type TabKey = "overview" | "sales" | "products" | "inventory" | "margins";

const tabs: TabKey[] = [
  "overview",
  "sales",
  "products",
  "inventory",
  "margins",
];

const presets = [
  { key: "today", label: "reports.today" },
  { key: "week", label: "reports.thisWeek" },
  { key: "month", label: "reports.thisMonth" },
  { key: "custom", label: "reports.custom" },
] as const;

const granularities = [
  { key: "day", label: "reports.granularityDay" },
  { key: "week", label: "reports.granularityWeek" },
  { key: "month", label: "reports.granularityMonth" },
] as const;

const loading = ref(false);
const exporting = ref(false);

const preset = ref<(typeof presets)[number]["key"]>("today");
const from = ref("");
const to = ref("");
const activeTab = ref<TabKey>("overview");

// ---------- data ----------
const summary = ref<SalesSummary | null>(null);
const trendPoints = ref<TrendPoint[]>([]);
const granularity = ref<(typeof granularities)[number]["key"]>("day");
const categories = ref<CategorySalesRow[]>([]);
const topProducts = ref<TopProductRow[]>([]);

const salesData = ref<SalesReportOutput | null>(null);
const salesFilters = ref({
  cashierId: null as number | null,
  customerId: null as number | null,
  includeVoided: false,
});
const cashiers = ref<UserRecord[]>([]);
const customerOptions = ref<CustomerLite[]>([]);

const inventory = ref<InventoryRow[]>([]);
const invSearch = ref("");
const invLowOnly = ref(false);

const marginData = ref<MarginReport | null>(null);

const showMovements = ref(false);
const movements = ref<StockMovement[]>([]);
const movementsLoading = ref(false);

const anyLoaded = computed(
  () =>
    summary.value !== null ||
    salesData.value !== null ||
    inventory.value.length > 0 ||
    marginData.value !== null,
);

function pad(n: number): string {
  return String(n).padStart(2, "0");
}
function isoDate(d: Date): string {
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

function applyPreset(key: (typeof presets)[number]["key"]) {
  preset.value = key;
  if (key === "custom") return;
  const now = new Date();
  to.value = isoDate(now);
  if (key === "today") {
    from.value = isoDate(now);
  } else if (key === "week") {
    const monday = new Date(now);
    const dow = (now.getDay() + 6) % 7;
    monday.setDate(now.getDate() - dow);
    from.value = isoDate(monday);
  } else {
    from.value = isoDate(new Date(now.getFullYear(), now.getMonth(), 1));
  }
}

function setPreset(key: (typeof presets)[number]["key"]) {
  applyPreset(key);
}

function customerLabel(c: CustomerLite): string {
  return c.phone ? `${c.name} (${c.phone})` : c.name;
}

function fmt(n: number): string {
  return formatMoney(n);
}

function num(n: number): string {
  return Number.isInteger(n)
    ? String(n)
    : n.toFixed(3).replace(/0+$/, "").replace(/\.$/, "");
}

function dateLabel(value: string): string {
  const d = new Date(
    value.replace(" ", "T") + (value.includes("Z") ? "" : "Z"),
  );
  return Number.isNaN(d.getTime())
    ? value
    : `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

// ---------- loaders ----------
async function loadOverview() {
  loading.value = true;
  try {
    const [s, tr, cats, tops] = await Promise.all([
      invoke<SalesSummary>("sales_summary", { from: from.value, to: to.value }),
      invoke<TrendPoint[]>("revenue_trend", {
        from: from.value,
        to: to.value,
        granularity: granularity.value,
      }),
      invoke<CategorySalesRow[]>("category_breakdown", {
        from: from.value,
        to: to.value,
      }),
      invoke<TopProductRow[]>("top_products", {
        from: from.value,
        to: to.value,
        limit: 100,
      }),
    ]);
    summary.value = s;
    trendPoints.value = tr;
    categories.value = cats;
    topProducts.value = tops;
    await nextTick();
    renderCharts();
  } catch (e) {
    toast.error(String(e));
  } finally {
    loading.value = false;
  }
}

async function loadSales() {
  loading.value = true;
  try {
    salesData.value = await invoke<SalesReportOutput>("sales_report", {
      from: from.value,
      to: to.value,
      cashierId: salesFilters.value.cashierId,
      customerId: salesFilters.value.customerId,
      includeVoided: salesFilters.value.includeVoided,
    });
  } catch (e) {
    toast.error(String(e));
  } finally {
    loading.value = false;
  }
}

async function loadInventory() {
  loading.value = true;
  try {
    inventory.value = await invoke<InventoryRow[]>("inventory_report");
  } catch (e) {
    toast.error(String(e));
  } finally {
    loading.value = false;
  }
}

async function loadMargins() {
  loading.value = true;
  try {
    marginData.value = await invoke<MarginReport>("margin_report", {
      from: from.value,
      to: to.value,
    });
  } catch (e) {
    toast.error(String(e));
  } finally {
    loading.value = false;
  }
}

async function loadMovements() {
  movementsLoading.value = true;
  try {
    movements.value = await invoke<StockMovement[]>("list_stock_movements");
  } catch (e) {
    toast.error(String(e));
  } finally {
    movementsLoading.value = false;
  }
}

function loadActiveTab() {
  if (!from.value || !to.value) return;
  if (activeTab.value === "overview") loadOverview();
  else if (activeTab.value === "sales") loadSales();
  else if (activeTab.value === "inventory") loadInventory();
  else if (activeTab.value === "margins") loadMargins();
  // products reuses topProducts loaded with overview; ensure it exists
  else if (!topProducts.value.length) loadOverview();
}

watch([from, to], () => loadActiveTab());
watch(granularity, () => {
  if (activeTab.value === "overview") loadOverview();
});
watch(activeTab, () => {
  nextTick(() => loadActiveTab());
});
watch(salesFilters, () => loadSales(), { deep: true });

// ---------- charts (F6.5 / F6.4 pie) ----------
const trendCanvas = ref<HTMLCanvasElement | null>(null);
const catCanvas = ref<HTMLCanvasElement | null>(null);
let trendChart: Chart | null = null;
let catChart: Chart | null = null;

function cssVar(name: string, fallback: string): string {
  const v = getComputedStyle(document.documentElement)
    .getPropertyValue(name)
    .trim();
  return v || fallback;
}

function renderCharts() {
  if (activeTab.value !== "overview") return;

  if (trendCanvas.value) {
    const labels = trendPoints.value.map((p) => p.bucket);
    const revenue = trendPoints.value.map((p) => Number(p.revenue.toFixed(2)));
    const accent = cssVar("--pos-accent", "#6366f1");
    if (trendChart) {
      trendChart.destroy();
      trendChart = null;
    }
    trendChart = new Chart(trendCanvas.value, {
      type: "bar",
      data: {
        labels,
        datasets: [
          {
            label: t("reports.kpiRevenue"),
            data: revenue,
            backgroundColor: accent,
            borderRadius: 6,
            maxBarThickness: 42,
          },
        ],
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        plugins: { legend: { display: false } },
        scales: {
          y: { beginAtZero: true },
          x: { ticks: { maxRotation: 0, autoSkipPadding: 12 } },
        },
      },
    });
  }

  if (catCanvas.value) {
    const palette = [
      "#6366f1",
      "#10b981",
      "#f59e0b",
      "#ef4444",
      "#06b6d4",
      "#a855f7",
      "#84cc16",
    ];
    if (catChart) {
      catChart.destroy();
      catChart = null;
    }
    catChart = new Chart(catCanvas.value, {
      type: "doughnut",
      data: {
        labels: categories.value.map((c) =>
          c.category === "—" ? t("reports.uncategorized") : c.category,
        ),
        datasets: [
          {
            data: categories.value.map((c) => Number(c.revenue.toFixed(2))),
            backgroundColor: categories.value.map(
              (_, i) => palette[i % palette.length],
            ),
            borderWidth: 0,
          },
        ],
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        plugins: { legend: { position: "bottom", labels: { boxWidth: 12 } } },
      },
    });
  }
}

// ---------- exports (F6.8) ----------
async function runExport(
  defaultName: string,
  command: string,
  args: Record<string, unknown>,
) {
  exporting.value = true;
  try {
    const path = await saveDialog({
      title: t("common.export"),
      defaultPath: defaultName,
      filters: [{ name: "Excel", extensions: ["xlsx"] }],
    });
    if (!path) return;
    await invoke(command, { path, ...args });
    toast.success(t("reports.exportedTo", { path }));
  } catch (e) {
    toast.error(String(e));
  } finally {
    exporting.value = false;
  }
}

const stamp = () => new Date().toISOString().slice(0, 10);

function exportSales() {
  runExport(`sales-${stamp()}.xlsx`, "export_sales_report", {
    from: from.value,
    to: to.value,
    cashierId: salesFilters.value.cashierId,
    customerId: salesFilters.value.customerId,
    includeVoided: salesFilters.value.includeVoided,
  });
}

function exportInventory() {
  runExport(`inventory-${stamp()}.xlsx`, "export_inventory", {});
}

function exportTopProducts() {
  runExport(`top-products-${stamp()}.xlsx`, "export_top_products", {
    from: from.value,
    to: to.value,
  });
}

// ---------- computed views ----------
const totalStockValue = computed(() =>
  inventory.value.reduce((sum, i) => sum + i.stockValue, 0),
);
const lowStockItems = computed(() => inventory.value.filter((i) => i.lowStock));

const filteredInventory = computed(() => {
  let rows = inventory.value;
  if (invLowOnly.value) rows = rows.filter((i) => i.lowStock);
  const q = invSearch.value.trim().toLowerCase();
  if (q) {
    rows = rows.filter((i) =>
      [i.name, i.category]
        .filter(Boolean)
        .some((v) => (v as string).toLowerCase().includes(q)),
    );
  }
  return rows;
});

// Client-side windowing: the whole report is fetched at once, so paginate in
// the UI to keep the table light.
const INV_PAGE_SIZE = 20;
const invVisibleCount = ref(INV_PAGE_SIZE);
const visibleInventory = computed(() =>
  filteredInventory.value.slice(0, invVisibleCount.value),
);

function showMoreInventory() {
  invVisibleCount.value += INV_PAGE_SIZE;
}

watch([invSearch, invLowOnly], () => {
  invVisibleCount.value = INV_PAGE_SIZE;
});

const salesRows = computed(() => salesData.value?.rows ?? []);

onMounted(async () => {
  applyPreset("today");
  await Promise.allSettled([settings.load()]);
  loadActiveTab();
  select<CustomerLite>(
    "SELECT id, name, phone FROM customers ORDER BY name",
  ).then((rows) => (customerOptions.value = rows));
  invoke<UserRecord[]>("list_users")
    .then((rows) => (cashiers.value = rows.filter((u) => u.isActive)))
    .catch(() => undefined);
});

onBeforeUnmount(() => {
  trendChart?.destroy();
  catChart?.destroy();
});

watch(showMovements, (open) => {
  if (open && !movements.value.length) loadMovements();
});
</script>
