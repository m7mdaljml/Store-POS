<script setup lang="ts">
import { computed, onMounted } from "vue";
import { RouterLink } from "vue-router";
import { useCatalogStore } from "../../stores/catalog";
import { convertFileSrc } from "@tauri-apps/api/core";

const catalog = useCatalogStore();

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
  await Promise.allSettled([catalog.load()]);
});
</script>

<template>
  <div>
    <h1 class="h4 mb-3">Dashboard</h1>

    <div class="row g-3 mb-3">
      <div class="col-6 col-lg-3">
        <div class="card p-3">
          <div class="text-muted small text-uppercase">Total Products</div>
          <div class="fs-4 fw-bold">{{ totalProducts }}</div>
        </div>
      </div>
      <div class="col-6 col-lg-3">
        <div class="card p-3">
          <div class="text-muted small text-uppercase">Active Products</div>
          <div class="fs-4 fw-bold">{{ activeProducts }}</div>
        </div>
      </div>
      <div class="col-6 col-lg-3">
        <div class="card p-3">
          <div class="text-muted small text-uppercase">Units on Hand</div>
          <div class="fs-4 fw-bold">{{ unitsOnHand }}</div>
        </div>
      </div>
      <div class="col-6 col-lg-3">
        <div class="card p-3" :class="lowStock.length ? 'border-danger' : ''">
          <div class="text-muted small text-uppercase">Low Stock</div>
          <div class="fs-4 fw-bold" :class="lowStock.length ? 'text-danger' : 'text-success'">
            {{ lowStock.length }}
          </div>
        </div>
      </div>
    </div>

    <div class="card">
      <div class="card-header d-flex align-items-center justify-content-between">
        <span class="fw-semibold">
          <i class="bi bi-exclamation-triangle me-1 text-warning"></i>Low-Stock Alerts
        </span>
        <RouterLink to="/stock" class="btn btn-sm btn-outline-secondary">
          <i class="bi bi-boxes me-1"></i>Go to Stock
        </RouterLink>
      </div>
      <div class="table-responsive">
        <table class="table align-middle mb-0">
          <thead>
            <tr>
              <th style="width: 52px"></th>
              <th>Product</th>
              <th>Category</th>
              <th class="text-end">In Stock</th>
              <th class="text-end">Reorder Level</th>
              <th>Status</th>
            </tr>
          </thead>
          <tbody>
            <tr v-if="!catalog.loaded">
              <td colspan="6" class="text-center text-muted py-4">Loading…</td>
            </tr>
            <tr v-else-if="!lowStock.length">
              <td colspan="6" class="text-center text-muted py-4">
                No products below their reorder level. Set a reorder level in Products →
                Edit to enable alerts.
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
                  {{ p.stock_qty === 0 ? "Out of stock" : "Low" }}
                </span>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  </div>
</template>
