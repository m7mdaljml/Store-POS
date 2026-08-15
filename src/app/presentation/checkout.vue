<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import { useCatalogStore } from "../../stores/catalog";
import { useSettingsStore } from "../../stores/settings";
import { useCartStore } from "../../stores/cart";
import { useScanner } from "../../composables/useScanner";
import type { Product } from "../../types";

const catalog = useCatalogStore();
const settings = useSettingsStore();
const cart = useCartStore();

const search = ref("");
const searchBox = ref<HTMLElement | null>(null);
const activeCategory = ref<number | null>(null);
const error = ref("");

const searchOpen = ref(false);
const activeSuggestion = ref(0);

const suggestions = computed(() => {
  const q = search.value.trim().toLowerCase();
  if (!q) return [];
  return catalog.products
    .filter((p) => {
      if (p.is_active !== 1) return false;
      if (activeCategory.value != null && p.category_id !== activeCategory.value)
        return false;
      return (
        p.name.toLowerCase().includes(q) ||
        (p.sku?.toLowerCase().includes(q) ?? false) ||
        (p.barcode?.toLowerCase().includes(q) ?? false)
      );
    })
    .slice(0, 8);
});

const filteredProducts = computed(() => {
  const q = search.value.trim().toLowerCase();
  return catalog.products.filter((p) => {
    if (p.is_active !== 1) return false;
    if (activeCategory.value != null && p.category_id !== activeCategory.value)
      return false;
    if (!q) return true;
    return (
      p.name.toLowerCase().includes(q) ||
      (p.sku?.toLowerCase().includes(q) ?? false) ||
      (p.barcode?.toLowerCase().includes(q) ?? false)
    );
  });
});

function fmt(n: number): string {
  if (!settings.currency) return n.toFixed(2);
  return new Intl.NumberFormat(undefined, {
    style: "currency",
    currency: settings.currency,
    currencyDisplay: "narrowSymbol",
  }).format(n);
}

function stockFor(productId: number): number {
  return catalog.products.find((p) => p.id === productId)?.stock_qty ?? 0;
}

function inCart(productId: number): number {
  return cart.items.find((i) => i.productId === productId)?.qty ?? 0;
}

function addToCart(p: Product) {
  if (p.stock_qty <= 0) {
    error.value = `"${p.name}" is out of stock`;
    return;
  }
  const inCartQty = inCart(p.id);
  if (inCartQty >= p.stock_qty) {
    error.value = `Only ${p.stock_qty} unit(s) of "${p.name}" in stock`;
    return;
  }
  error.value = "";
  cart.add({ productId: p.id, name: p.name, price: p.sell_price, qty: 1, discount: 0 });
}

function addSuggestion(p: Product) {
  addToCart(p);
  search.value = "";
  searchOpen.value = false;
}

function onSearchInput() {
  activeSuggestion.value = 0;
  searchOpen.value = search.value.trim().length > 0 && suggestions.value.length > 0;
}

function onSearchKeydown(e: KeyboardEvent) {
  if (e.key === "Escape") {
    searchOpen.value = false;
    return;
  }
  if (e.key === "ArrowDown") {
    e.preventDefault();
    searchOpen.value = true;
    if (suggestions.value.length) {
      activeSuggestion.value =
        (activeSuggestion.value + 1) % suggestions.value.length;
    }
    return;
  }
  if (e.key === "ArrowUp") {
    e.preventDefault();
    if (suggestions.value.length) {
      activeSuggestion.value =
        (activeSuggestion.value - 1 + suggestions.value.length) %
        suggestions.value.length;
    }
    return;
  }
  if (e.key === "Enter") {
    const choice = suggestions.value[activeSuggestion.value];
    if (choice) {
      e.preventDefault();
      addSuggestion(choice);
    }
  }
}

function pickSuggestion(p: Product) {
  addSuggestion(p);
}

function onDocClick(e: MouseEvent) {
  if (searchBox.value && !searchBox.value.contains(e.target as Node)) {
    searchOpen.value = false;
  }
}

onMounted(async () => {
  document.addEventListener("click", onDocClick);
  await Promise.allSettled([
    catalog.loaded ? Promise.resolve() : catalog.load(),
    settings.loaded ? Promise.resolve() : settings.load(),
  ]);
});

onBeforeUnmount(() => {
  document.removeEventListener("click", onDocClick);
});

function bumpQty(productId: number, delta: number) {
  const item = cart.items.find((i) => i.productId === productId);
  if (!item) return;
  const next = item.qty + delta;
  if (next <= 0) {
    cart.remove(productId);
    return;
  }
  if (next > stockFor(productId)) {
    error.value = `Only ${stockFor(productId)} unit(s) of "${item.name}" in stock`;
    return;
  }
  error.value = "";
  cart.setQty(productId, next);
}

function onItemDiscount(productId: number, e: Event) {
  const value = Number((e.target as HTMLInputElement).value);
  cart.setDiscount(productId, isNaN(value) || value < 0 ? 0 : value);
}

function onOrderDiscount(e: Event) {
  const value = Number((e.target as HTMLInputElement).value);
  cart.orderDiscount = isNaN(value) || value < 0 ? 0 : value;
}

useScanner({
  onScan: (code) => {
    const product = catalog.products.find(
      (p) => p.is_active === 1 && (p.barcode === code || p.sku === code)
    );
    if (!product) {
      error.value = `No product found for barcode "${code}"`;
      return;
    }
    addToCart(product);
    search.value = "";
    searchOpen.value = false;
  },
});

onMounted(async () => {
  await Promise.allSettled([
    catalog.loaded ? Promise.resolve() : catalog.load(),
    settings.loaded ? Promise.resolve() : settings.load(),
  ]);
});
</script>

<template>
  <div class="checkout-grid">
    <section class="checkout-left">
      <div class="mb-3 checkout-search" ref="searchBox">
        <input
          v-model="search"
          class="form-control form-control-lg"
          type="search"
          placeholder="Search by product name, SKU or barcode…"
          aria-label="Search products"
          autocomplete="off"
          @input="onSearchInput"
          @focus="onSearchInput"
          @keydown="onSearchKeydown"
        />
        <div v-if="searchOpen && suggestions.length" class="checkout-search-dropdown card">
          <button
            v-for="(s, i) in suggestions"
            :key="s.id"
            class="search-item"
            :class="{ active: i === activeSuggestion }"
            type="button"
            @mouseenter="activeSuggestion = i"
            @click="pickSuggestion(s)"
          >
            <span class="search-item-name">
              {{ s.name }}
              <span class="text-muted small ms-1">
                {{ s.sku || s.barcode }}
              </span>
            </span>
            <span class="ms-auto text-nowrap">
              <span class="fw-semibold">{{ fmt(s.sell_price) }}</span>
              <span class="text-muted small ms-2">
                {{ s.stock_qty }} {{ s.unit }}
              </span>
            </span>
          </button>
          <div v-if="!suggestions.length" class="text-muted small p-3">
            No matching products
          </div>
        </div>
      </div>

      <div class="checkout-cat-tabs mb-3">
        <button
          class="cat-tab"
          :class="{ active: activeCategory == null }"
          type="button"
          @click="activeCategory = null"
        >
          All
        </button>
        <button
          v-for="c in catalog.categories"
          :key="c.id"
          class="cat-tab"
          :class="{ active: activeCategory === c.id }"
          type="button"
          @click="activeCategory = c.id"
        >
          {{ c.name }}
        </button>
      </div>

      <div v-if="error" class="alert alert-warning py-1 px-2 mb-3 small" role="alert">
        <i class="bi bi-exclamation-triangle me-1"></i>{{ error }}
      </div>

      <div class="product-grid">
        <p v-if="!filteredProducts.length" class="text-muted py-4 text-center w-100">
          No products found
        </p>
        <button
          v-for="p in filteredProducts"
          :key="p.id"
          class="product-card"
          type="button"
          :title="p.stock_qty > 0 ? 'Click to add to cart' : 'Out of stock'"
          @click="addToCart(p)"
        >
          <div class="product-card-img">
            <img v-if="p.image_path" :src="convertFileSrc(p.image_path)" alt="" />
            <i v-else class="bi bi-box-seam"></i>
          </div>
          <div class="product-card-name">{{ p.name }}</div>
          <div class="product-card-price">{{ fmt(p.sell_price) }}</div>
          <div class="product-card-stock" :class="{ 'text-danger': p.stock_qty <= 0 }">
            <i class="bi bi-box me-1"></i>{{ p.stock_qty }} {{ p.unit }}
          </div>
        </button>
      </div>
    </section>

    <aside class="checkout-right card">
      <div class="card-header d-flex align-items-center justify-content-between">
        <span class="fw-semibold">
          <i class="bi bi-cart3 me-2"></i>Cart
          <span class="badge text-bg-primary ms-1">{{ cart.itemCount }}</span>
        </span>
        <button
          class="btn btn-sm btn-outline-secondary"
          type="button"
          :disabled="!cart.items.length"
          @click="cart.clear()"
        >
          <i class="bi bi-trash me-1"></i>Clear
        </button>
      </div>

      <div class="cart-items">
        <p v-if="!cart.items.length" class="text-muted small text-center my-5">
          Cart is empty — click products to add them
        </p>
        <div v-for="item in cart.items" :key="item.productId" class="cart-item">
          <div class="d-flex justify-content-between gap-2">
            <div class="fw-semibold small">{{ item.name }}</div>
            <button
              class="btn btn-sm btn-link text-danger p-0 lh-1"
              type="button"
              title="Remove"
              @click="cart.remove(item.productId)"
            >
              <i class="bi bi-x-lg"></i>
            </button>
          </div>
          <div class="d-flex justify-content-between align-items-center mt-1">
            <div class="input-group input-group-sm cart-qty">
              <button
                class="btn btn-outline-secondary"
                type="button"
                @click="bumpQty(item.productId, -1)"
              >
                <i class="bi bi-dash"></i>
              </button>
              <span class="input-group-text cart-qty-val">{{ item.qty }}</span>
              <button
                class="btn btn-outline-secondary"
                type="button"
                :disabled="item.qty >= stockFor(item.productId)"
                @click="bumpQty(item.productId, 1)"
              >
                <i class="bi bi-plus"></i>
              </button>
            </div>
            <div class="text-end">
              <div class="fw-semibold">{{ fmt(cart.lineTotal(item)) }}</div>
              <div class="text-muted" style="font-size: 0.72rem">
                {{ fmt(item.price) }} each
              </div>
            </div>
          </div>
          <div class="d-flex justify-content-between align-items-center mt-1">
            <span class="text-muted" style="font-size: 0.72rem">Discount / unit</span>
            <div class="input-group input-group-sm item-discount">
              <span class="input-group-text">−</span>
              <input
                class="form-control text-end"
                type="number"
                min="0"
                step="0.01"
                :max="item.price"
                :value="item.discount"
                :aria-label="`Discount per unit for ${item.name}`"
                @input="onItemDiscount(item.productId, $event)"
              />
            </div>
          </div>
        </div>
      </div>

      <div class="cart-totals card-body border-top">
        <div class="d-flex justify-content-between align-items-center mb-2">
          <label class="form-label mb-0 small text-muted" for="order-discount">
            Order discount
          </label>
          <div class="input-group input-group-sm" style="width: 150px">
            <span class="input-group-text">−</span>
            <input
              id="order-discount"
              class="form-control text-end"
              type="number"
              min="0"
              step="0.01"
              :value="cart.orderDiscount"
              aria-label="Order discount amount"
              @input="onOrderDiscount"
            />
          </div>
        </div>

        <div class="d-flex justify-content-between mb-1">
          <span class="text-muted">Subtotal</span>
          <span>{{ fmt(cart.subtotal) }}</span>
        </div>
        <div v-if="cart.itemDiscountTotal" class="d-flex justify-content-between mb-1 text-danger">
          <span class="text-muted">Item discounts</span>
          <span>−{{ fmt(cart.itemDiscountTotal) }}</span>
        </div>
        <div v-if="cart.orderDiscount" class="d-flex justify-content-between mb-1 text-danger">
          <span class="text-muted">Order discount</span>
          <span>−{{ fmt(cart.orderDiscount) }}</span>
        </div>
        <div class="d-flex justify-content-between align-items-center pt-2 border-top">
          <span class="fw-semibold">Total</span>
          <span class="fs-4 fw-bold">{{ fmt(cart.total) }}</span>
        </div>
      </div>

      <div class="checkout-payment card-body border-top">
        <div class="text-muted small mb-2">
          <i class="bi bi-credit-card me-1"></i>Payment methods arrive in the next step
        </div>
        <button class="btn btn-lg btn-primary w-100" type="button" disabled>
          <i class="bi bi-cash-coin me-2"></i>Complete Sale
        </button>
      </div>
    </aside>
  </div>
</template>
