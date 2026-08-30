<template>
  <div class="checkout-page">
    <div
      v-if="!openSession"
      class="alert alert-warning py-2 small d-flex align-items-center gap-2 mb-3"
      role="alert"
    >
      <i class="bi bi-exclamation-triangle-fill"></i>
      <span>{{ t("checkout.registerClosed") }}</span>
      <RouterLink
        to="/settings"
        class="btn btn-sm btn-warning ms-auto text-nowrap"
      >
        <i class="bi bi-box-arrow-in-right mx-1"></i
        >{{ t("checkout.openRegister") }}
      </RouterLink>
    </div>

    <div
      class="checkout-grid"
      :class="{ 'cart-only': cartOnly, 'quick-collapsed': quickCollapsed }"
    >
      <div class="mb-3 checkout-search" ref="searchBox">
        <i class="bi bi-upc-scan checkout-scan-icon"></i>
        <div class="checkout-search-row">
          <input
            v-model="search"
            class="form-control form-control-lg checkout-search-input"
            type="search"
            :placeholder="t('checkout.searchPlaceholder') + ' (F2)'"
            :aria-label="t('checkout.searchPlaceholder')"
            autocomplete="off"
            @input="onSearchInput"
            @focus="onSearchInput"
            @keydown="onSearchKeydown"
          />
          <button
            v-if="!cartOnly"
            class="btn btn-soft quick-items-toggle"
            type="button"
            :title="t('checkout.toggleQuickItems')"
            :aria-label="t('checkout.toggleQuickItems')"
            :aria-expanded="!quickCollapsed"
            @click="quickCollapsed = !quickCollapsed"
          >
            <i
              class="bi"
              :class="
                quickCollapsed
                  ? 'bi-chevron-double-left'
                  : 'bi-chevron-double-right'
              "
            ></i>
          </button>
        </div>
        <div
          v-if="searchOpen && suggestions.length"
          class="checkout-search-dropdown card"
        >
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
                {{ s.barcode }}
              </span>
            </span>
            <span class="ms-auto text-nowrap">
              <span class="fw-semibold">{{ fmt(sellWithTax(s)) }}</span>
              <span class="text-muted small ms-2">
                {{ s.stock_qty }} {{ s.unit }}
              </span>
            </span>
          </button>
        </div>
      </div>

      <section class="checkout-left">
        <div class="product-grid">
          <p
            v-if="!filteredProducts.length && search.trim()"
            class="text-muted py-4 text-center w-100"
          >
            {{ t("checkout.noProducts") }}
          </p>
          <button
            v-for="p in filteredProducts"
            :key="p.id"
            class="product-card"
            :class="{
              'is-oos': p.stock_qty <= 0,
              'is-in-cart': inCart(p.id) > 0,
            }"
            type="button"
            :disabled="p.stock_qty <= 0"
            :title="
              p.stock_qty > 0
                ? t('checkout.clickToAdd')
                : t('checkout.oosBadge')
            "
            @click="addToCart(p)"
          >
            <div class="product-card-img">
              <img
                v-if="p.image_path"
                :src="convertFileSrc(p.image_path)"
                alt=""
              />
              <i v-else class="bi bi-box-seam"></i>
              <span v-if="p.stock_qty <= 0" class="product-card-oos">{{
                t("checkout.oosBadge")
              }}</span>
            </div>
            <div class="product-card-name">{{ p.name }}</div>
            <div class="product-card-foot">
              <span class="product-card-price">{{ fmt(sellWithTax(p)) }}</span>
              <span v-if="inCart(p.id)" class="product-card-badge">
                {{ inCart(p.id) }}
              </span>
              <span
                class="stock-pill"
                :class="
                  p.stock_qty <= 0
                    ? 'out'
                    : p.stock_qty <= p.reorder_level
                      ? 'low'
                      : 'in'
                "
              >
                {{ p.stock_qty }} {{ p.unit }}
              </span>
            </div>
          </button>
        </div>
      </section>

      <aside class="ticket-panel">
        <header class="ticket-header">
          <div class="d-flex align-items-center gap-2">
            <i class="bi bi-basket2-fill ticket-header-icon"></i>
            <span class="ticket-title">{{ t("checkout.cart") }}</span>
            <span v-if="cart.itemCount" class="ticket-count">{{
              cart.itemCount
            }}</span>
          </div>
          <button
            class="btn btn-sm btn-soft"
            type="button"
            :disabled="!cart.items.length"
            :title="t('common.clear')"
            @click="cart.clear()"
          >
            <i class="bi bi-trash"></i>
          </button>
        </header>

        <div class="ticket-customer">
          <details ref="customerDetails" class="customer-picker">
            <summary>
              <i class="bi bi-person-circle"></i>
              <span class="text-muted small">{{ t("common.customer") }}</span>
              <strong v-if="selectedCustomer" class="cust-name">
                {{ customerLabel(selectedCustomer) }}
              </strong>
              <strong v-else class="text-muted">{{
                t("checkout.walkIn")
              }}</strong>
              <i class="bi bi-chevron-down ms-auto picker-caret"></i>
            </summary>
            <div class="customer-menu card">
              <input
                v-model="customerQuery"
                class="form-control form-control-sm mb-2"
                type="search"
                :placeholder="t('checkout.searchCustomers')"
                :aria-label="t('checkout.searchCustomers')"
              />
              <div class="customer-list">
                <button
                  v-if="!filteredPickCustomers.length"
                  class="customer-option text-muted"
                  type="button"
                  disabled
                >
                  {{ t("customers.noMatching") }}
                </button>
                <button
                  v-for="c in filteredPickCustomers"
                  :key="c.id"
                  class="customer-option"
                  :class="{ selected: c.id === selectedCustomerId }"
                  type="button"
                  @click="pickCustomer(c.id)"
                >
                  <span class="text-truncate">{{ customerLabel(c) }}</span>
                  <span
                    v-if="c.balance > 0.005"
                    class="badge text-bg-danger rounded-pill flex-shrink-0"
                  >
                    {{ fmt(c.balance) }}
                  </span>
                </button>
              </div>
            </div>
          </details>
        </div>

        <div class="ticket-items">
          <div v-if="!cart.items.length" class="ticket-empty">
            <i class="bi bi-basket2"></i>
            <p>{{ t("checkout.cartEmpty") }}</p>
          </div>

          <div
            v-for="item in cart.items"
            :key="item.productId"
            class="ticket-line"
          >
            <div class="ticket-line-head">
              <span class="ticket-line-name" :title="item.name">{{
                item.name
              }}</span>
              <span class="ticket-line-unit"
                >{{ fmt(cart.unitPriceWithTax(item)) }}
                {{ t("checkout.eachUnit") }}</span
              >
              <button
                class="line-remove"
                type="button"
                :title="t('common.remove')"
                @click="cart.remove(item.productId)"
              >
                <i class="bi bi-x-lg"></i>
              </button>
            </div>
            <div class="ticket-line-body">
              <div class="line-stepper">
                <button
                  type="button"
                  aria-label="-"
                  @click="bumpQty(item.productId, -1)"
                >
                  <i class="bi bi-dash"></i>
                </button>
                <span>{{ item.qty }}</span>
                <button
                  type="button"
                  aria-label="+"
                  :disabled="item.qty >= stockFor(item.productId)"
                  @click="bumpQty(item.productId, 1)"
                >
                  <i class="bi bi-plus"></i>
                </button>
              </div>
              <input
                class="form-control form-control-sm line-disc"
                type="number"
                min="0"
                step="1"
                placeholder="0.00"
                :max="item.price"
                :value="item.discount"
                :aria-label="`${t('checkout.discountPerUnit')} — ${item.name}`"
                @input="onItemDiscount(item.productId, $event)"
              />
              <span class="line-total">{{ fmt(cart.lineTotalWithTax(item)) }}</span>
            </div>
          </div>
        </div>

        <footer class="ticket-footer">
          <div class="ticket-discount">
            <label
              class="form-label mb-0 small text-muted"
              for="order-discount"
            >
              {{ t("checkout.orderDiscount") }}
            </label>
            <div class="input-group input-group-sm disc-input">
              <input
                id="order-discount"
                class="form-control text-end"
                type="number"
                placeholder="0.00"
                :min="0"
                :max="discountMax()"
                step="1"
                :value="cart.orderDiscountValue"
                :aria-label="
                  cart.orderDiscountType === 'percent'
                    ? t('checkout.discountPercentAria')
                    : t('checkout.discountAmountAria')
                "
                @input="onOrderDiscount"
              />
              <span
                v-if="cart.orderDiscountType === 'percent'"
                class="input-group-text"
                >%</span
              >
            </div>
            <div
              class="btn-group btn-group-sm ms-auto"
              role="group"
              :aria-label="t('checkout.discountType')"
            >
              <button
                class="btn"
                :class="
                  cart.orderDiscountType === 'fixed'
                    ? 'btn-primary'
                    : 'btn-soft'
                "
                type="button"
                @click="setDiscountType('fixed')"
              >
                {{ t("checkout.fixed") }}
              </button>
              <button
                class="btn"
                :class="
                  cart.orderDiscountType === 'percent'
                    ? 'btn-primary'
                    : 'btn-soft'
                "
                type="button"
                @click="setDiscountType('percent')"
              >
                %
              </button>
            </div>
          </div>
          <div class="text-muted mb-2" style="font-size: 0.72rem">
            {{ discountLimitHint }}
          </div>

          <div class="t-row">
            <span>{{ t("checkout.subtotal") }}</span>
            <span class="fw-semibold">{{ fmt(cart.subtotal) }}</span>
          </div>
          <div v-if="cart.itemDiscountTotal" class="t-row t-disc">
            <span>{{ t("checkout.itemDiscounts") }}</span>
            <span>−{{ fmt(cart.itemDiscountTotal) }}</span>
          </div>
          <div v-if="cart.orderDiscountAmount" class="t-row t-disc">
            <span>{{ t("checkout.orderDiscount") }}</span>
            <span>
              −{{ fmt(cart.orderDiscountAmount) }}
              <span
                v-if="cart.orderDiscountType === 'percent'"
                class="text-muted"
              >
                ({{ cart.orderDiscountValue }}%)
              </span>
            </span>
          </div>
          <div v-if="cart.tax" class="t-row">
            <span>{{ t("checkout.tax") }}</span>
            <span class="fw-semibold">{{ fmt(cart.tax) }}</span>
          </div>
          <div class="t-total-row">
            <span>{{ t("common.total") }}</span>
            <span class="grand-total">{{ fmt(cart.total) }}</span>
          </div>

          <div v-if="currencies.length > 1" class="t-conv-row">
            <AppSelect
              v-model="displayCurrencyId"
              sm
              class="app-select-inline"
              :items="currencies"
              :option-label="(c) => `${c.code} (${c.symbol})`"
              :option-value="(c) => c.id"
              :aria-label="t('checkout.displayCurrency')"
            />
            <span
              v-if="convertedTotal !== null"
              class="t-conv-note small text-muted ms-auto"
            >
              ≈ {{ convertedTotal.toFixed(2) }} {{ selectedCurrency?.symbol }}
            </span>
          </div>

          <div class="ticket-pay">
            <div class="mb-2">
              <label
                class="form-label mb-1 small text-muted"
                for="cash-received"
                >{{ t("checkout.cashReceived") }} <small>(F5)</small></label
              >
              <div class="input-group">
                <input
                  id="cash-received"
                  class="form-control text-end fs-5 fw-semibold"
                  type="number"
                  min="0"
                  step="1"
                  :value="cart.cashReceived"
                  :aria-label="t('checkout.cashReceived')"
                  @input="onCashReceived"
                />
                <span class="input-group-text">{{
                  baseCurrencySymbol ||
                  settings.currency ||
                  t("checkout.currencyFallback")
                }}</span>
                <button
                  class="btn btn-outline-secondary"
                  type="button"
                  :title="t('checkout.exactTitle')"
                  @click="cart.syncCashToTotal()"
                >
                  {{ t("checkout.exact") }}
                </button>
              </div>
            </div>
            <div
              v-if="quickCashAmounts.length"
              class="d-flex flex-wrap gap-1 mb-2"
              :aria-label="t('checkout.quickCashAria')"
            >
              <button
                v-for="amt in quickCashAmounts"
                :key="amt"
                class="btn btn-sm btn-quick"
                type="button"
                :title="`${t('checkout.cashReceived')} ${fmt(amt)}`"
                @click="cart.cashReceived = amt"
              >
                {{ fmt(amt) }}
              </button>
            </div>

            <button
              v-if="selectedCustomerId"
              class="btn btn-soft w-100 mb-2"
              type="button"
              :disabled="cart.shortfall <= 0.005"
              :title="`${t('customers.balance')}: ${fmt(selectedCustomer?.balance ?? 0)}`"
              @click="chargeToTab"
            >
              <i class="bi bi-journal-plus mx-1"></i>
              {{ t("checkout.chargeToTab", { amount: fmt(cart.shortfall) }) }}
            </button>

            <details
              class="split-details mb-2"
              :open="cart.splitLines.length > 0"
            >
              <summary>
                <i class="bi bi-credit-card-2-front mx-1"></i
                >{{ t("checkout.splitPayments") }}
                <span
                  v-if="cart.splitLines.length"
                  class="badge text-bg-secondary ms-1"
                  >{{ cart.splitLines.length }}</span
                >
              </summary>
              <div class="mt-2">
                <div
                  v-for="(line, i) in cart.splitLines"
                  :key="i"
                  class="payment-line"
                >
                  <select
                    class="form-select form-select-sm payment-line-method"
                    :value="line.method"
                    :aria-label="`${t('common.method')} ${i + 1}`"
                    @change="
                      cart.setSplitLine(i, {
                        method: ($event.target as HTMLSelectElement)
                          .value as PaymentLine['method'],
                      })
                    "
                  >
                    <option value="card">{{ t("checkout.card") }}</option>
                    <option value="credit">
                      {{ t("checkout.customerCredit") }}
                    </option>
                  </select>
                  <template v-if="line.method === 'credit'">
                    <AppSelect
                      sm
                      :model-value="line.customerId"
                      :items="customers"
                      :option-label="customerLabel"
                      :option-value="(c) => c.id"
                      :placeholder="t('checkout.selectCustomer')"
                      :aria-label="`${t('checkout.customerCredit')} ${i + 1}`"
                      @update:model-value="
                        (v) =>
                          cart.setSplitLine(i, {
                            customerId: v == null ? null : Number(v),
                          })
                      "
                    />
                  </template>
                  <template v-else>
                    <input
                      class="form-control form-control-sm"
                      type="text"
                      :value="line.reference ?? ''"
                      :placeholder="t('checkout.cardRef')"
                      :aria-label="`${t('checkout.cardRef')} ${i + 1}`"
                      @input="
                        cart.setSplitLine(i, {
                          reference: ($event.target as HTMLInputElement).value,
                        })
                      "
                    />
                  </template>
                  <div class="input-group input-group-sm">
                    <input
                      class="form-control text-end"
                      type="number"
                      min="0"
                      step="1"
                      :value="line.amount"
                      :aria-label="`${t('common.amount')} ${i + 1}`"
                      @input="onSplitAmount(i, $event)"
                    />
                    <span class="input-group-text">{{
                      baseCurrencySymbol ||
                      settings.currency ||
                      t("checkout.currencyFallback")
                    }}</span>
                  </div>
                  <button
                    class="btn btn-sm btn-outline-danger"
                    type="button"
                    :title="t('common.remove')"
                    @click="cart.removeSplitLine(i)"
                  >
                    <i class="bi bi-x-lg"></i>
                  </button>
                </div>

                <button
                  class="btn btn-sm btn-soft w-100 mt-1"
                  type="button"
                  @click="cart.addSplitLine()"
                >
                  <i class="bi bi-plus-lg mx-1"></i
                  >{{ t("checkout.addPayment") }}
                </button>

                <div class="t-row mt-2">
                  <span>{{ t("checkout.splitPayments") }}</span>
                  <span class="fw-semibold">{{ fmt(cart.splitTotal) }}</span>
                </div>
              </div>
            </details>

            <div v-if="cart.cashReceived > 0" class="t-row">
              <span>{{ t("checkout.cashTendered") }}</span>
              <span class="fw-semibold">{{ fmt(cart.cashReceived) }}</span>
            </div>
            <div
              v-if="cart.shortfall > 0.005"
              class="pay-banner pay-banner-danger"
            >
              <i class="bi bi-exclamation-circle mx-1"></i>
              <span>{{ t("checkout.short") }}</span>
              <strong class="ms-auto">−{{ fmt(cart.shortfall) }}</strong>
            </div>
            <div
              v-if="cart.change > 0.005"
              class="pay-banner pay-banner-success"
            >
              <i class="bi bi-arrow-return-left mx-1"></i>
              <span>{{ t("checkout.change") }}</span>
              <strong class="ms-auto">{{ fmt(cart.change) }}</strong>
            </div>
          </div>

          <div class="ticket-actions">
            <div class="d-flex align-items-center gap-2 mb-2">
              <button
                class="btn btn-soft flex-fill"
                type="button"
                :disabled="
                  !cart.items.length || !openSession || holding || committing
                "
                :title="t('checkout.hold')"
                @click="holdCurrentSale"
                @keydown="onSearchKeydown"
              >
                <span
                  v-if="holding"
                  class="spinner-border spinner-border-sm mx-1"
                  role="status"
                ></span>
                <i v-else class="bi bi-pause-circle mx-1"></i
                >{{ t("checkout.hold") }}
                <small class="text-muted">(F12)</small>
              </button>
              <button
                class="btn btn-soft flex-fill"
                type="button"
                @click="openHeldSales"
              >
                <i class="bi bi-clock-history mx-1"></i
                >{{ t("checkout.heldSales") }}
              </button>
              <div class="form-check mb-0 text-nowrap">
                <input
                  id="print-receipt"
                  v-model="printReceiptOnComplete"
                  class="form-check-input"
                  type="checkbox"
                />
                <label class="form-check-label small" for="print-receipt">{{
                  t("checkout.printReceiptAfter")
                }}</label>
              </div>
            </div>

            <button
              class="btn btn-pay w-100"
              type="button"
              :disabled="
                !cart.items.length ||
                !cart.paymentValid ||
                !openSession ||
                committing
              "
              @click="completeSale"
            >
              <span
                v-if="committing"
                class="spinner-border spinner-border-sm"
                role="status"
              ></span>
              <i v-else class="bi bi-cash-stack"></i>
              <span
                >{{ t("checkout.completeSale") }}
                <small class="text-muted">(F4)</small></span
              >
              <span class="btn-pay-amt">{{ fmt(cart.total) }}</span>
            </button>
          </div>
        </footer>
      </aside>

      <!-- Sale confirmation modal -->
      <div
        v-if="saleConfirmOpen"
        class="modal fade show d-block"
        tabindex="-1"
        role="dialog"
        @click.self="saleConfirmOpen = false"
      >
        <div class="modal-dialog modal-dialog-centered" role="document">
          <div class="modal-content">
            <div class="modal-header">
              <h5 class="modal-title">
                <i class="bi bi-cash-stack mx-2"></i
                >{{ t("checkout.completeSale") }}
              </h5>
              <button
                type="button"
                class="btn-close"
                @click="saleConfirmOpen = false"
              ></button>
            </div>
            <div class="modal-body text-center py-4">
              <p class="fs-5 mb-2">
                {{ t("checkout.confirmSale", { total: fmt(cart.total) }) }}
              </p>
              <div class="d-flex justify-content-center gap-2 mt-3">
                <button
                  type="button"
                  class="btn btn-outline-secondary"
                  @click="saleConfirmOpen = false"
                >
                  {{ t("common.cancel") }}
                </button>
                <button
                  type="button"
                  class="btn btn-success"
                  @click="submitSale"
                >
                  <i class="bi bi-check-lg mx-1"></i
                  >{{ t("checkout.completeSale") }}
                  <small class="text-white-50">(F4)</small>
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>

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
                <i class="bi bi-clock-history mx-2"></i
                >{{ t("checkout.heldSales") }}
              </h5>
              <button
                type="button"
                class="btn-close"
                :aria-label="t('common.close')"
                @click="heldSalesOpen = false"
              ></button>
            </div>
            <div class="modal-body">
              <div
                v-if="heldError"
                class="alert alert-warning py-1 px-2 small"
                role="alert"
              >
                {{ heldError }}
              </div>
              <p
                v-if="!heldSales.length && !heldError"
                class="text-muted small text-center my-4"
              >
                {{ t("checkout.heldNone") }}
              </p>
              <div
                v-for="sale in heldSales"
                :key="sale.id"
                class="d-flex justify-content-between align-items-center border-bottom py-2"
              >
                <div>
                  <div class="fw-semibold small">{{ sale.saleNo }}</div>
                  <div class="text-muted" style="font-size: 0.72rem">
                    {{
                      t(
                        "checkout.itemCount",
                        { count: sale.itemCount },
                        sale.itemCount,
                      )
                    }}
                    · {{ fmt(sale.total) }} ·
                    {{ dateLabel(sale.createdAt) }}
                  </div>
                </div>
                <div class="d-flex gap-2">
                  <button
                    class="btn btn-sm btn-primary"
                    type="button"
                    :disabled="resumingHeld"
                    @click="resumeHeldSale(sale)"
                  >
                    {{ t("checkout.resume") }}
                  </button>
                  <button
                    class="btn btn-sm btn-outline-danger"
                    type="button"
                    :disabled="resumingHeld"
                    @click="cancelHeldSale(sale)"
                  >
                    {{ t("common.cancel") }}
                  </button>
                </div>
              </div>
            </div>
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
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { useI18n } from "vue-i18n";
import { RouterLink } from "vue-router";
import AppSelect from "../../components/AppSelect.vue";
import { useCatalogStore } from "../../stores/catalog";
import { useSettingsStore } from "../../stores/settings";
import { useCartStore } from "../../stores/cart";
import { useAuth } from "../../composables/useAuth";
import type { Paged } from "../../composables/usePagedList";
import { useConfirm } from "../../composables/useConfirm";
import { useScanner } from "../../composables/useScanner";
import { useToast } from "../../composables/useToast";
import { select } from "../../lib/db";
import { playBeep } from "../../lib/sound";
import { buildReceiptHtml, printReceipt } from "../../lib/receipt";
import { baseCurrencySymbol, formatMoney } from "../../lib/currency";
import type {
  Currency,
  CustomerLite,
  HoldSaleResult,
  PaymentLine,
  Product,
  ResumeSaleRecord,
  SaleReceipt,
  SaleRecord,
  SaleResult,
  SaleSession,
} from "../../types";

const catalog = useCatalogStore();
const settings = useSettingsStore();
const cart = useCartStore();
const auth = useAuth();
const { confirmDialog } = useConfirm();
const toast = useToast();
const { t, locale } = useI18n();

const customers = ref<CustomerLite[]>([]);
/** Currencies available for a display-only total conversion (F7.3). */
const currencies = ref<Currency[]>([]);
const displayCurrencyId = ref<number | null>(null);

const selectedCurrency = computed(
  () => currencies.value.find((c) => c.id === displayCurrencyId.value) ?? null,
);
const baseCurrency = computed(
  () => currencies.value.find((c) => c.is_base === 1) ?? null,
);
/**
 * Grand total expressed in the selected display currency. Rates are stored
 * relative to the base currency (base rate = 1); sales are always recorded in
 * the base currency, this is a read-only convenience conversion.
 */
const convertedTotal = computed(() => {
  const cur = selectedCurrency.value;
  if (!cur || cur.is_base === 1) return null;
  const baseRate = baseCurrency.value?.rate || 1;
  return (cart.total / baseRate) * cur.rate;
});

const selectedCustomerId = ref<number | null>(null);
const customerQuery = ref("");
const customerDetails = ref<HTMLDetailsElement | null>(null);

const search = ref("");
const searchBox = ref<HTMLElement | null>(null);
const activeCategory = ref<number | null>(null);

const searchOpen = ref(false);
const activeSuggestion = ref(0);

// Quick items panel: manually collapsible while browsing; searching always
// re-expands it so results are visible.
const quickCollapsed = ref(false);
watch(search, (q) => {
  if (q.trim()) quickCollapsed.value = false;
});

const committing = ref(false);
const printReceiptOnComplete = ref(false);

const openSession = ref<SaleSession | null>(null);
const saleConfirmOpen = ref(false);

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
  // Search matches any active product in the system.
  return catalog.products
    .filter((p) => {
      if (p.is_active !== 1) return false;
      return (
        p.name.toLowerCase().includes(q) ||
        (p.barcode?.toLowerCase().includes(q) ?? false)
      );
    })
    .slice(0, 8);
});
const filteredProducts = computed(() => {
  const q = search.value.trim().toLowerCase();
  return catalog.products.filter((p) => {
    if (p.is_active !== 1) return false;
    if (!q) {
      // Idle grid: only quick items, scoped by the selected category tab.
      return (
        p.is_quick === 1 &&
        (activeCategory.value == null || p.category_id === activeCategory.value)
      );
    }
    // Searching: any item in the system can appear.
    return (
      p.name.toLowerCase().includes(q) ||
      (p.barcode?.toLowerCase().includes(q) ?? false)
    );
  });
});

/**
 * Nothing quick to browse while idle: collapse the product panel and let the
 * cart span the full width. The scan/search bar always stays visible.
 */
const cartOnly = computed(
  () => !search.value.trim() && filteredProducts.value.length === 0,
);

function fmt(n: number): string {
  return formatMoney(n);
}

/** Product sell price including tax. */
function sellWithTax(p: { sell_price: number; tax_rate: number | null }): number {
  return p.sell_price * (1 + (p.tax_rate ?? 0) / 100);
}

function dateLabel(d: string): string {
  const date = new Date(d + (d.includes("T") ? "" : "Z"));
  if (isNaN(date.getTime())) return d;
  return date.toLocaleString(locale.value);
}

function stockFor(productId: number): number {
  return catalog.products.find((p) => p.id === productId)?.stock_qty ?? 0;
}

function inCart(productId: number): number {
  return cart.items.find((i) => i.productId === productId)?.qty ?? 0;
}

function addToCart(p: Product) {
  if (p.stock_qty <= 0) {
    toast.error(t("checkout.outOfStock", { name: p.name }));
    return;
  }
  const inCartQty = inCart(p.id);
  if (inCartQty >= p.stock_qty) {
    toast.error(
      t("checkout.stockLimit", {
        stock: p.stock_qty,
        name: p.name,
      }),
    );
    return;
  }
  cart.add({
    productId: p.id,
    name: p.name,
    price: p.sell_price,
    qty: 1,
    discount: 0,
    costPrice: p.cost_price ?? 0,
    taxRate: p.tax_rate ?? 0,
  });
  playBeep("click");
}

function addSuggestion(p: Product) {
  addToCart(p);
  search.value = "";
  searchOpen.value = false;
}

function onSearchInput() {
  activeSuggestion.value = 0;
  searchOpen.value =
    search.value.trim().length > 0 && suggestions.value.length > 0;
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

const selectedCustomer = computed(
  () => customers.value.find((c) => c.id === selectedCustomerId.value) ?? null,
);

const filteredPickCustomers = computed(() => {
  const q = customerQuery.value.trim().toLowerCase();
  if (!q) return customers.value;
  return customers.value.filter((c) =>
    [c.name, c.phone]
      .filter(Boolean)
      .some((v) => (v as string).toLowerCase().includes(q)),
  );
});

function pickCustomer(id: number | null) {
  selectedCustomerId.value = id;
  customerQuery.value = "";
  if (customerDetails.value) customerDetails.value.open = false;
}

const quickCashAmounts = computed(() => {
  const total = cart.total;
  if (total <= 0) return [] as number[];
  const amounts = new Set<number>();
  for (const step of [5, 10, 20, 50]) {
    amounts.add(Math.ceil(total / step) * step);
  }
  return [...amounts].filter((a) => a > total);
});

function onCashReceived(e: Event) {
  const value = Number((e.target as HTMLInputElement).value);
  cart.cashReceived = isNaN(value) || value < 0 ? 0 : value;
}

function onSplitAmount(index: number, e: Event) {
  const value = Number((e.target as HTMLInputElement).value);
  cart.setSplitLine(index, { amount: isNaN(value) || value < 0 ? 0 : value });
}

function chargeToTab() {
  if (!selectedCustomerId.value) return;
  const amount = Number(cart.shortfall.toFixed(2));
  if (amount <= 0.004) return;
  const idx = cart.splitLines.findIndex((l) => l.method === "credit");
  if (idx >= 0) {
    cart.setSplitLine(idx, { customerId: selectedCustomerId.value, amount });
  } else {
    cart.addSplitLine({
      method: "credit",
      customerId: selectedCustomerId.value,
      amount,
    });
  }
}

function completeSale() {
  if (!cart.items.length) return;
  if (!cart.paymentValid) {
    toast.error(t("checkout.paymentShort", { amount: fmt(cart.shortfall) }));
    return;
  }
  if (cart.splitLines.some((l) => l.method === "credit" && !l.customerId)) {
    toast.error(t("checkout.creditNeedsCustomer"));
    return;
  }
  if (!openSession.value) {
    toast.error(t("checkout.registerRequired"));
    return;
  }
  saleConfirmOpen.value = true;
}

async function submitSale() {
  saleConfirmOpen.value = false;
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
          ? [
              {
                method: "cash",
                amount: cart.cashReceived,
                reference: null,
                customerId: null,
              },
            ]
          : []),
        ...cart.splitLines.map((l) => ({
          method: l.method,
          amount: l.amount,
          reference: l.reference ?? null,
          customerId: l.customerId ?? null,
        })),
      ],
      discount: cart.orderDiscountAmount,
      tax: cart.tax,
      customerId: heldCustomerId.value ?? selectedCustomerId.value,
      userId: auth.user?.id ?? null,
      heldSaleId: heldSaleId.value,
      sessionId: openSession.value?.id ?? null,
    },
  })
    .then((sale) => {
      const hadCredit = cart.splitLines.some((l) => l.method === "credit");
      const change = fmt(sale.changeGiven);
      toast.success(
        t("checkout.saleCompleted", {
          saleNo: sale.saleNo,
          change,
        }),
      );
      playBeep("success");
      cart.clear();
      heldSaleId.value = null;
      heldCustomerId.value = null;
      selectedCustomerId.value = null;
      catalog.load();
      loadOpenSession();
      if (hadCredit) loadCustomers();
      if (printReceiptOnComplete.value) {
        printSaleReceipt(sale.saleId).catch((e: unknown) => {
          toast.error(t("checkout.printFailed", { error: String(e) }));
        });
      }
    })
    .catch((e: string) => {
      toast.error(String(e));
    })
    .finally(() => {
      committing.value = false;
    });
}

async function loadOpenSession() {
  try {
    openSession.value = await invoke<SaleSession | null>("get_open_session");
  } catch (e) {
    toast.error(String(e));
  }
}

async function printSaleReceipt(saleId: number) {
  const receipt = await invoke<SaleReceipt>("get_sale_receipt", {
    input: { saleId },
  });
  await printReceipt(buildReceiptHtml(receipt));
}

function holdCurrentSale() {
  if (!cart.items.length) return;
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
      tax: cart.tax,
      customerId: null,
      userId: auth.user?.id ?? null,
    },
  })
    .then((held) => {
      toast.success(t("checkout.saleHeld", { saleNo: held.saleNo }));
      cart.clear();
      heldSaleId.value = null;
      heldCustomerId.value = null;
      selectedCustomerId.value = null;
    })
    .catch((e: string) => {
      toast.error(String(e));
    })
    .finally(() => {
      holding.value = false;
    });
}

async function loadHeldSales() {
  heldError.value = "";
  try {
    heldSales.value = (
      await invoke<Paged<SaleRecord>>("list_sales", {
        input: { status: "held", limit: null },
      })
    ).items;
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
    toast.error(t("checkout.clearCartFirst"));
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
          taxRate: li.taxRate ?? 0,
        }),
      );
      cart.setOrderDiscount("fixed", record.discount);
      heldSaleId.value = record.saleId;
      heldCustomerId.value = record.customerId;
      heldSalesOpen.value = false;
      toast.success(t("checkout.heldLoaded", { saleNo: record.saleNo }));
      search.value = "";
    })
    .catch((e: string) => {
      toast.error(String(e));
    })
    .finally(() => {
      resumingHeld.value = false;
    });
}

async function cancelHeldSale(sale: SaleRecord) {
  if (
    !(await confirmDialog({
      message: t("checkout.cancelHeld", { saleNo: sale.saleNo }),
    }))
  )
    return;
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
      cart.cashReceived = Number(t.toFixed(2));
    }
    lastTotal = t;
  },
);

onMounted(async () => {
  document.addEventListener("click", onDocClick);
  document.addEventListener("pointerdown", onDocPointerDown);
  window.addEventListener("keydown", onGlobalKeydown);
  lastTotal = cart.total;
  await Promise.allSettled([
    catalog.loaded ? Promise.resolve() : catalog.load(),
    settings.loaded ? Promise.resolve() : settings.load(),
    loadOpenSession(),
    loadCustomers(),
    loadCurrencies(),
  ]);
  nextTick(() => focusSearch(false));
});

function loadCurrencies(): Promise<void> {
  return select<Currency>(
    "SELECT id, code, name, symbol, rate, is_base FROM currencies ORDER BY is_base DESC, code",
  )
    .then((rows) => {
      currencies.value = rows;
      if (displayCurrencyId.value === null) {
        const base = rows.find((c) => c.is_base === 1);
        if (base) displayCurrencyId.value = base.id;
      }
    })
    .then(() => undefined);
}

function loadCustomers(): Promise<void> {
  return select<CustomerLite>(
    "SELECT id, name, phone, balance FROM customers ORDER BY name",
  )
    .then((rows) => (customers.value = rows))
    .then(() => undefined);
}

onBeforeUnmount(() => {
  document.removeEventListener("click", onDocClick);
  document.removeEventListener("pointerdown", onDocPointerDown);
  window.removeEventListener("keydown", onGlobalKeydown);
});

/**
 * F9.5 keyboard shortcuts:
 *   F2  → new sale: focus the product search box
 *   F4  → complete sale
 *   F5  → payment: focus the cash-received field
 *   Esc → cancel: close popovers / clear transient errors
 *   F12 → hold the current sale
 */
function onGlobalKeydown(e: KeyboardEvent) {
  const target = e.target as HTMLElement | null;
  const inTextField =
    target &&
    (target.tagName === "INPUT" ||
      target.tagName === "TEXTAREA" ||
      target.tagName === "SELECT");

  switch (e.key) {
    case "F2": {
      e.preventDefault();
      focusSearch(true);
      break;
    }
    case "F4": {
      e.preventDefault();
      if (saleConfirmOpen.value) {
        submitSale();
      } else if (!committing.value) {
        completeSale();
      }
      break;
    }
    case "F5": {
      e.preventDefault();
      document.getElementById("cash-received")?.focus();
      break;
    }
    case "F12": {
      // Don't hijack F12 while typing; otherwise hold the sale.
      if (!inTextField && cart.items.length) {
        e.preventDefault();
        holdCurrentSale();
      }
      break;
    }
    case "Escape": {
      if (!searchOpen.value) return;
      searchOpen.value = false;
      break;
    }
  }
}

function bumpQty(productId: number, delta: number) {
  const item = cart.items.find((i) => i.productId === productId);
  if (!item) return;
  const next = item.qty + delta;
  if (next <= 0) {
    cart.remove(productId);
    return;
  }
  if (next > stockFor(productId)) {
    toast.error(
      t("checkout.stockLimit", {
        stock: stockFor(productId),
        name: item.name,
      }),
    );
    return;
  }
  cart.setQty(productId, next);
}

function onItemDiscount(productId: number, e: Event) {
  const item = cart.items.find((i) => i.productId === productId);
  if (!item) return;
  const value = Number((e.target as HTMLInputElement).value);
  if (isNaN(value) || value < 0) {
    toast.error(t("checkout.invalidDiscount"));
    return;
  }
  const amount = value * item.qty;
  if (!discountAllowed(amount)) {
    toast.error(discountError(amount));
    return;
  }
  cart.setDiscount(productId, value);
}

/** True when the user may apply a discount of `amount` (admin = unlimited). */
function discountAllowed(amount: number): boolean {
  if (auth.can("sales.discount")) return true;
  return amount <= settings.discountThreshold;
}

function discountError(amount: number): string {
  return t("checkout.discountTooHigh", {
    amount: fmt(amount),
    limit: fmt(settings.discountThreshold),
  });
}

const discountLimitHint = computed(() =>
  auth.can("sales.discount")
    ? t("checkout.discountUnlimited")
    : t("checkout.discountLimited", {
        amount: fmt(settings.discountThreshold),
      }),
);

function discountMax(): number {
  if (cart.orderDiscountType === "percent") return 100;
  return cart.subtotal;
}

function onOrderDiscount(e: Event) {
  const value = Number((e.target as HTMLInputElement).value);
  if (isNaN(value) || value < 0) {
    toast.error(t("checkout.invalidOrderDiscount"));
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
    toast.error(discountError(amount));
    return;
  }
  cart.setOrderDiscount(type, value);
}

useScanner({
  burstMs: 100,
  onScan: (code) => {
    const product = catalog.products.find(
      (p) => p.is_active === 1 && p.barcode === code,
    );
    if (!product) {
      toast.error(t("checkout.noProductBarcode", { code }));
      return;
    }
    addToCart(product);
    search.value = "";
    searchOpen.value = false;
    focusSearch(false);
  },
});

/**
 * Keeps the scan/search input focused like a POS terminal: any click that
 * does not target a real typing field (input/textarea/select/contenteditable)
 * and does not happen while a modal is open returns focus to the scan box,
 * so barcode scanner input always lands in the right place.
 */
function focusSearch(select = true) {
  const input = searchBox.value?.querySelector<HTMLInputElement>("input");
  if (!input || document.activeElement === input) return;
  input.focus();
  if (select) input.select();
}

function scanFocusTargetOk(target: EventTarget | null): boolean {
  const el = target as HTMLElement | null;
  if (!el || typeof el.closest !== "function") return false;
  if (el.closest("input, textarea, select, [contenteditable='true']"))
    return false;
  if (document.querySelector(".modal-backdrop.show, .modal.d-block"))
    return false;
  return true;
}

function onDocPointerDown(e: PointerEvent) {
  if (!scanFocusTargetOk(e.target)) return;
  requestAnimationFrame(() => focusSearch(false));
}
</script>

<style scoped>
.checkout-page {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 420px;
}

.checkout-page .checkout-grid {
  flex: 1;
  height: auto;
  min-height: 0;
}

.app-select-inline {
  width: 180px;
}
</style>
