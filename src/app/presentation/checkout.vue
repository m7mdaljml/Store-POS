<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { useCatalogStore } from "../../stores/catalog";
import { useSettingsStore } from "../../stores/settings";
import { useCartStore } from "../../stores/cart";
import { useAuth } from "../../composables/useAuth";
import { useScanner } from "../../composables/useScanner";
import { select } from "../../lib/db";
import type {
  CustomerLite,
  HoldSaleResult,
  PaymentLine,
  Product,
  ResumeSaleRecord,
  SaleRecord,
  SaleResult,
} from "../../types";

const catalog = useCatalogStore();
const settings = useSettingsStore();
const cart = useCartStore();
const auth = useAuth();

const customers = ref<CustomerLite[]>([]);

const search = ref("");
const searchBox = ref<HTMLElement | null>(null);
const activeCategory = ref<number | null>(null);
const error = ref("");

const searchOpen = ref(false);
const activeSuggestion = ref(0);

const notice = ref("");
const committing = ref(false);

const heldSaleId = ref<number | null>(null);
const heldCustomerId = ref<number | null>(null);
const heldSales = ref<SaleRecord[]>([]);
const heldSalesOpen = ref(false);
const holding = ref(false);
const resumingHeld = ref(false);
const heldError = ref("");

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
  cart.add({
    productId: p.id,
    name: p.name,
    price: p.sell_price,
    qty: 1,
    discount: 0,
    costPrice: p.cost_price ?? 0,
  });
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

function customerLabel(c: CustomerLite): string {
  return c.phone ? `${c.name} (${c.phone})` : c.name;
}

function onCashReceived(e: Event) {
  const value = Number((e.target as HTMLInputElement).value);
  cart.cashReceived = isNaN(value) || value < 0 ? 0 : value;
}

function onSplitAmount(index: number, e: Event) {
  const value = Number((e.target as HTMLInputElement).value);
  cart.setSplitLine(index, { amount: isNaN(value) || value < 0 ? 0 : value });
}

function completeSale() {
  if (!cart.items.length) return;
  if (!cart.paymentValid) {
    error.value = `Payment is short by ${fmt(cart.shortfall)}`;
    return;
  }
  error.value = "";
  notice.value = "";
  committing.value = true;
  invoke<SaleResult>("create_sale", {
    input: {
      items: cart.items.map((i) => ({
        productId: i.productId,
        qty: i.qty,
        price: i.price,
        costPrice: i.costPrice,
        discount: i.discount,
      })),
      payments: [
        ...(cart.cashReceived > 0
          ? [{ method: "cash", amount: cart.cashReceived, reference: null, customerId: null }]
          : []),
        ...cart.splitLines.map((l) => ({
          method: l.method,
          amount: l.amount,
          reference: l.reference ?? null,
          customerId: l.customerId ?? null,
        })),
      ],
      discount: cart.orderDiscountAmount,
      tax: 0,
      customerId: heldCustomerId.value,
      userId: auth.user?.id ?? null,
      heldSaleId: heldSaleId.value,
    },
  })
    .then((sale) => {
      const change = fmt(sale.changeGiven);
      notice.value = `Sale ${sale.saleNo} completed. Change due: ${change}.`;
      cart.clear();
      heldSaleId.value = null;
      heldCustomerId.value = null;
      catalog.load();
    })
    .catch((e: string) => {
      error.value = String(e);
    })
    .finally(() => {
      committing.value = false;
    });
}

function holdCurrentSale() {
  if (!cart.items.length) return;
  error.value = "";
  notice.value = "";
  holding.value = true;
  invoke<HoldSaleResult>("hold_sale", {
    input: {
      items: cart.items.map((i) => ({
        productId: i.productId,
        qty: i.qty,
        price: i.price,
        costPrice: i.costPrice,
        discount: i.discount,
      })),
      discount: cart.orderDiscountAmount,
      tax: 0,
      customerId: null,
      userId: auth.user?.id ?? null,
    },
  })
    .then((held) => {
      notice.value = `Sale ${held.saleNo} held — resume it anytime from the held sales list.`;
      cart.clear();
      heldSaleId.value = null;
      heldCustomerId.value = null;
    })
    .catch((e: string) => {
      error.value = String(e);
    })
    .finally(() => {
      holding.value = false;
    });
}

async function loadHeldSales() {
  heldError.value = "";
  try {
    heldSales.value = await invoke<SaleRecord[]>("list_sales", {
      input: { status: "held", limit: null },
    });
  } catch (e) {
    heldError.value = String(e);
  }
}

function openHeldSales() {
  heldSalesOpen.value = true;
  loadHeldSales();
}

function resumeHeldSale(sale: SaleRecord) {
  if (cart.items.length) {
    error.value = "Clear the current cart before resuming a held sale";
    return;
  }
  resumingHeld.value = true;
  heldError.value = "";
  invoke<ResumeSaleRecord>("resume_sale", {
    input: { saleId: sale.id, userId: auth.user?.id ?? null },
  })
    .then((record) => {
      record.items.forEach((li) =>
        cart.add({
          productId: li.productId,
          name: li.name,
          price: li.price,
          qty: li.qty,
          discount: li.discount,
          costPrice: li.costPrice,
        })
      );
      cart.setOrderDiscount("fixed", record.discount);
      heldSaleId.value = record.saleId;
      heldCustomerId.value = record.customerId;
      heldSalesOpen.value = false;
      notice.value = `Held sale ${record.saleNo} loaded into the cart.`;
      search.value = "";
    })
    .catch((e: string) => {
      error.value = String(e);
    })
    .finally(() => {
      resumingHeld.value = false;
    });
}

function cancelHeldSale(sale: SaleRecord) {
  if (!confirm(`Cancel held sale ${sale.saleNo}? This cannot be undone.`)) return;
  resumingHeld.value = true;
  invoke("cancel_held_sale", {
    input: { saleId: sale.id, userId: auth.user?.id ?? null },
  })
    .then(() => loadHeldSales())
    .catch((e: string) => {
      heldError.value = String(e);
    })
    .finally(() => {
      resumingHeld.value = false;
    });
}

let lastTotal = cart.total;
watch(
  () => cart.total,
  (t) => {
    if (Math.abs(cart.cashReceived - lastTotal) < 0.005) {
      cart.cashReceived = t;
    }
    lastTotal = t;
  }
);

onMounted(async () => {
  document.addEventListener("click", onDocClick);
  lastTotal = cart.total;
  await Promise.allSettled([
    catalog.loaded ? Promise.resolve() : catalog.load(),
    settings.loaded ? Promise.resolve() : settings.load(),
    select<CustomerLite>("SELECT id, name, phone, balance FROM customers ORDER BY name").then(
      (rows) => (customers.value = rows)
    ),
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
  const item = cart.items.find((i) => i.productId === productId);
  if (!item) return;
  const value = Number((e.target as HTMLInputElement).value);
  if (isNaN(value) || value < 0) {
    error.value = "Enter a valid item discount";
    return;
  }
  const amount = value * item.qty;
  if (!discountAllowed(amount)) {
    error.value = discountError(amount);
    return;
  }
  error.value = "";
  cart.setDiscount(productId, value);
}

/** True when the user may apply a discount of `amount` (admin = unlimited). */
function discountAllowed(amount: number): boolean {
  if (auth.can("sales.discount")) return true;
  return amount <= settings.discountThreshold;
}

function discountError(amount: number): string {
  return `Discount of ${fmt(amount)} exceeds the ${fmt(settings.discountThreshold)} limit — requires sales.discount permission`;
}

const discountLimitHint = computed(() =>
  auth.can("sales.discount")
    ? "Unlimited discounts (admin)"
    : `Limited to ${fmt(settings.discountThreshold)} per order`
);

function discountMax(): number {
  if (cart.orderDiscountType === "percent") return 100;
  return cart.subtotal;
}

function onOrderDiscount(e: Event) {
  const value = Number((e.target as HTMLInputElement).value);
  if (isNaN(value) || value < 0) {
    error.value = "Enter a valid discount";
    return;
  }
  applyOrderDiscount(cart.orderDiscountType, value);
}

function setDiscountType(type: "fixed" | "percent") {
  applyOrderDiscount(type, cart.orderDiscountValue);
}

function applyOrderDiscount(type: "fixed" | "percent", value: number) {
  const amount =
    type === "percent"
      ? Math.min(cart.subtotal, (cart.subtotal * value) / 100)
      : Math.min(cart.subtotal, value);
  if (!discountAllowed(amount)) {
    error.value = discountError(amount);
    return;
  }
  error.value = "";
  cart.setOrderDiscount(type, value);
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
      <div v-if="notice" class="alert alert-success py-1 px-2 mb-3 small" role="alert">
        <i class="bi bi-check-circle me-1"></i>{{ notice }}
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
          <div class="btn-group btn-group-sm" role="group" aria-label="Discount type">
            <button
              class="btn btn-outline-secondary"
              :class="{ active: cart.orderDiscountType === 'fixed' }"
              type="button"
              @click="setDiscountType('fixed')"
            >
              Fixed
            </button>
            <button
              class="btn btn-outline-secondary"
              :class="{ active: cart.orderDiscountType === 'percent' }"
              type="button"
              @click="setDiscountType('percent')"
            >
              %
            </button>
          </div>
        </div>
        <div class="input-group input-group-sm mb-1">
          <span class="input-group-text">−</span>
          <input
            id="order-discount"
            class="form-control text-end"
            type="number"
            :min="0"
            :max="discountMax()"
            step="0.01"
            :value="cart.orderDiscountValue"
            :aria-label="
              cart.orderDiscountType === 'percent'
                ? 'Order discount percentage'
                : 'Order discount amount'
            "
            @input="onOrderDiscount"
          />
          <span v-if="cart.orderDiscountType === 'percent'" class="input-group-text">%</span>
        </div>
        <div class="text-muted mb-2" style="font-size: 0.72rem">{{ discountLimitHint }}</div>

        <div class="d-flex justify-content-between mb-1">
          <span class="text-muted">Subtotal</span>
          <span>{{ fmt(cart.subtotal) }}</span>
        </div>
        <div v-if="cart.itemDiscountTotal" class="d-flex justify-content-between mb-1 text-danger">
          <span class="text-muted">Item discounts</span>
          <span>−{{ fmt(cart.itemDiscountTotal) }}</span>
        </div>
        <div
          v-if="cart.orderDiscountAmount"
          class="d-flex justify-content-between mb-1 text-danger"
        >
          <span class="text-muted">Order discount</span>
          <span>
            −{{ fmt(cart.orderDiscountAmount) }}
            <span v-if="cart.orderDiscountType === 'percent'" class="text-muted">
              ({{ cart.orderDiscountValue }}%)
            </span>
          </span>
        </div>
        <div class="d-flex justify-content-between align-items-center pt-2 border-top">
          <span class="fw-semibold">Total</span>
          <span class="fs-4 fw-bold">{{ fmt(cart.total) }}</span>
        </div>
      </div>

      <div class="checkout-payment card-body border-top">
        <div class="fw-semibold small mb-2">
          <i class="bi bi-cash-coin me-1"></i>Payment
        </div>

        <div class="mb-2">
          <label class="form-label mb-1 small text-muted" for="cash-received">Cash received</label>
          <div class="input-group input-group-sm">
            <span class="input-group-text">{{ settings.currency || "amt" }}</span>
            <input
              id="cash-received"
              class="form-control text-end"
              type="number"
              min="0"
              step="0.01"
              :value="cart.cashReceived"
              aria-label="Cash received"
              @input="onCashReceived"
            />
            <button
              class="btn btn-outline-secondary"
              type="button"
              title="Set cash to the exact total"
              @click="cart.syncCashToTotal()"
            >
              Exact
            </button>
          </div>
        </div>

        <div v-if="cart.splitLines.length" class="mb-2">
          <div class="text-muted small mb-1">Split payments</div>
          <div
            v-for="(line, i) in cart.splitLines"
            :key="i"
            class="payment-line"
          >
            <select
              class="form-select form-select-sm payment-line-method"
              :value="line.method"
              :aria-label="`Payment ${i + 1} method`"
              @change="
                cart.setSplitLine(i, {
                  method: ($event.target as HTMLSelectElement).value as PaymentLine['method'],
                })
              "
            >
              <option value="card">Card</option>
              <option value="credit">Customer credit</option>
            </select>
            <template v-if="line.method === 'credit'">
              <select
                class="form-select form-select-sm"
                :value="line.customerId ?? ''"
                :aria-label="`Payment ${i + 1} customer`"
                @change="
                  cart.setSplitLine(i, {
                    customerId: ($event.target as HTMLSelectElement).value
                      ? Number(($event.target as HTMLSelectElement).value)
                      : null,
                  })
                "
              >
                <option value="" disabled>Select customer…</option>
                <option v-for="c in customers" :key="c.id" :value="c.id">
                  {{ customerLabel(c) }}
                </option>
              </select>
            </template>
            <template v-else>
              <input
                class="form-control form-control-sm"
                type="text"
                :value="line.reference ?? ''"
                placeholder="Card ref (optional)"
                :aria-label="`Payment ${i + 1} card reference`"
                @input="
                  cart.setSplitLine(i, {
                    reference: ($event.target as HTMLInputElement).value,
                  })
                "
              />
            </template>
            <div class="input-group input-group-sm">
              <span class="input-group-text">{{ settings.currency || "amt" }}</span>
              <input
                class="form-control text-end"
                type="number"
                min="0"
                step="0.01"
                :value="line.amount"
                :aria-label="`Payment ${i + 1} amount`"
                @input="onSplitAmount(i, $event)"
              />
            </div>
            <button
              class="btn btn-sm btn-outline-danger"
              type="button"
              title="Remove payment"
              @click="cart.removeSplitLine(i)"
            >
              <i class="bi bi-x-lg"></i>
            </button>
          </div>
        </div>

        <button
          class="btn btn-sm btn-outline-secondary w-100 mb-2"
          type="button"
          @click="cart.addSplitLine()"
        >
          <i class="bi bi-plus-lg me-1"></i>Add payment method
        </button>

        <div class="d-flex justify-content-between mb-1">
          <span class="text-muted">Total</span>
          <span class="fw-semibold">{{ fmt(cart.total) }}</span>
        </div>
        <div class="d-flex justify-content-between mb-1">
          <span class="text-muted">Split payments</span>
          <span>{{ fmt(cart.splitTotal) }}</span>
        </div>
        <div class="d-flex justify-content-between mb-1">
          <span class="text-muted">Cash tendered</span>
          <span>{{ fmt(cart.cashReceived) }}</span>
        </div>
        <div
          v-if="cart.shortfall > 0.005"
          class="d-flex justify-content-between mb-1 text-danger fw-semibold"
        >
          <span>Short</span>
          <span>−{{ fmt(cart.shortfall) }}</span>
        </div>
        <div
          v-if="cart.change > 0.005"
          class="d-flex justify-content-between mb-2 text-success fw-semibold"
        >
          <span>Change</span>
          <span>{{ fmt(cart.change) }}</span>
        </div>

        <div class="d-flex gap-2 mb-2">
          <button
            class="btn btn-outline-secondary flex-fill"
            type="button"
            :disabled="!cart.items.length || holding || committing"
            title="Save the cart to resume later"
            @click="holdCurrentSale"
          >
            <span v-if="holding" class="spinner-border spinner-border-sm me-1" role="status"></span>
            <i v-else class="bi bi-pause-circle me-1"></i>Hold
          </button>
          <button
            class="btn btn-outline-secondary flex-fill"
            type="button"
            @click="openHeldSales"
          >
            <i class="bi bi-clock-history me-1"></i>Held Sales
          </button>
        </div>

        <button
          class="btn btn-lg btn-primary w-100"
          type="button"
          :disabled="!cart.items.length || !cart.paymentValid || committing"
          @click="completeSale"
        >
          <span v-if="committing" class="spinner-border spinner-border-sm me-2" role="status"></span>
          <i v-else class="bi bi-cash-coin me-2"></i>Complete Sale
        </button>
      </div>
    </aside>

    <div
      v-if="heldSalesOpen"
      class="modal fade show d-block"
      tabindex="-1"
      role="dialog"
      @click.self="heldSalesOpen = false"
    >
      <div class="modal-dialog" role="document">
        <div class="modal-content">
          <div class="modal-header">
            <h5 class="modal-title">
              <i class="bi bi-clock-history me-2"></i>Held sales
            </h5>
            <button
              type="button"
              class="btn-close"
              aria-label="Close"
              @click="heldSalesOpen = false"
            ></button>
          </div>
          <div class="modal-body">
            <div v-if="heldError" class="alert alert-warning py-1 px-2 small" role="alert">
              {{ heldError }}
            </div>
            <p v-if="!heldSales.length && !heldError" class="text-muted small text-center my-4">
              No held sales
            </p>
            <div
              v-for="sale in heldSales"
              :key="sale.id"
              class="d-flex justify-content-between align-items-center border-bottom py-2"
            >
              <div>
                <div class="fw-semibold small">{{ sale.saleNo }}</div>
                <div class="text-muted" style="font-size: 0.72rem">
                  {{ sale.itemCount }} item(s) · {{ fmt(sale.total) }} ·
                  {{ new Date(sale.createdAt).toLocaleString() }}
                </div>
              </div>
              <div class="d-flex gap-2">
                <button
                  class="btn btn-sm btn-primary"
                  type="button"
                  :disabled="resumingHeld"
                  @click="resumeHeldSale(sale)"
                >
                  Resume
                </button>
                <button
                  class="btn btn-sm btn-outline-danger"
                  type="button"
                  :disabled="resumingHeld"
                  @click="cancelHeldSale(sale)"
                >
                  Cancel
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
