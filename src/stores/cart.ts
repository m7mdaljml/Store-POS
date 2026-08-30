import { computed, ref } from "vue";
import { defineStore } from "pinia";
import type { CartItem, PaymentLine } from "../types";

export const useCartStore = defineStore("cart", () => {
  const items = ref<CartItem[]>([]);
  const orderDiscountType = ref<"fixed" | "percent">("fixed");
  const orderDiscountValue = ref(0);

  const subtotal = computed(() =>
    items.value.reduce((sum, item) => sum + item.price * item.qty, 0)
  );
  const itemDiscountTotal = computed(() =>
    items.value.reduce((sum, item) => sum + item.discount * item.qty, 0)
  );
  const orderDiscountAmount = computed(() => {
    const value = orderDiscountValue.value;
    if (value <= 0 || !subtotal.value) return 0;
    if (orderDiscountType.value === "percent") {
      return Math.min(subtotal.value, (subtotal.value * value) / 100);
    }
    return Math.min(subtotal.value, value);
  });
  /** Order tax = sum of each line (price - discount) * qty * taxRate / 100. */
  const tax = computed(() => {
    const raw = items.value.reduce(
      (sum, item) => sum + Math.max(0, item.price - item.discount) * item.qty * (item.taxRate ?? 0) / 100,
      0
    );
    return Math.max(0, raw);
  });
  const total = computed(() =>
    Math.max(
      0,
      subtotal.value - itemDiscountTotal.value - orderDiscountAmount.value + tax.value
    )
  );
  const itemCount = computed(() =>
    items.value.reduce((sum, item) => sum + item.qty, 0)
  );

  function add(item: CartItem) {
    const existing = items.value.find(
      (i) => i.productId === item.productId
    );
    if (existing) existing.qty += item.qty;
    else items.value.push({ ...item });
  }

  function remove(productId: number) {
    items.value = items.value.filter((i) => i.productId !== productId);
  }

  function setQty(productId: number, qty: number) {
    const item = items.value.find((i) => i.productId === productId);
    if (item) item.qty = Math.max(0, qty);
  }

  /** Per-unit discount, clamped to [0, unit price]. */
  function setDiscount(productId: number, discount: number) {
    const item = items.value.find((i) => i.productId === productId);
    if (!item) return;
    item.discount = Math.min(Math.max(0, discount), item.price);
  }

  /** Line total after item discount: (price - discount) * qty, floor 0. */
  function lineTotal(item: CartItem): number {
    return Math.max(0, item.price - item.discount) * item.qty;
  }

  /** Unit price including tax (tax-inclusive display price). */
  function unitPriceWithTax(item: CartItem): number {
    return item.price * (1 + (item.taxRate ?? 0) / 100);
  }

  /** Line total including tax: (price - discount) * qty * (1 + rate/100). */
  function lineTotalWithTax(item: CartItem): number {
    return lineTotal(item) * (1 + (item.taxRate ?? 0) / 100);
  }

  /** Order-level discount; type is "fixed" (currency) or "percent" of subtotal. */
  function setOrderDiscount(type: "fixed" | "percent", value: number) {
    orderDiscountType.value = type;
    orderDiscountValue.value = Math.max(0, value);
  }

  /* ----------------------------- Payment ----------------------------- */

  const cashReceived = ref(0);
  const splitLines = ref<PaymentLine[]>([]);

  const splitTotal = computed(() =>
    splitLines.value.reduce(
      (sum, line) => sum + (isNaN(line.amount) ? 0 : line.amount),
      0
    )
  );
  /** Amount that still needs to be paid after split payments. */
  const remainder = computed(() => Math.max(0, total.value - splitTotal.value));
  /** Cash change back to the customer. */
  const change = computed(() => Math.max(0, +(cashReceived.value - remainder.value).toFixed(2)));
  /** Amount still outstanding on the order. */
  const shortfall = computed(() => Math.max(0, +(remainder.value - cashReceived.value).toFixed(2)));
  const paymentValid = computed(() => shortfall.value <= 0.005);

  function syncCashToTotal() {
    cashReceived.value = Number(total.value.toFixed(2));
  }

  function addSplitLine(patch: Partial<PaymentLine> = {}) {
    splitLines.value.push({
      method: patch.method ?? "card",
      amount: patch.amount ?? 0,
      reference: patch.reference ?? null,
      customerId: patch.customerId ?? null,
    });
  }

  function removeSplitLine(index: number) {
    splitLines.value.splice(index, 1);
  }

  function setSplitLine(index: number, patch: Partial<PaymentLine>) {
    const line = splitLines.value[index];
    if (line) Object.assign(line, patch);
  }

  function clear() {
    items.value = [];
    orderDiscountType.value = "fixed";
    orderDiscountValue.value = 0;
    cashReceived.value = 0;
    splitLines.value = [];
  }

  return {
    items,
    orderDiscountType,
    orderDiscountValue,
    orderDiscountAmount,
    subtotal,
    itemDiscountTotal,
    total,
    tax,
    itemCount,
    cashReceived,
    splitLines,
    splitTotal,
    remainder,
    change,
    shortfall,
    paymentValid,
    add,
    remove,
    setQty,
    setDiscount,
    lineTotal,
    unitPriceWithTax,
    lineTotalWithTax,
    setOrderDiscount,
    syncCashToTotal,
    addSplitLine,
    removeSplitLine,
    setSplitLine,
    clear,
  };
});
