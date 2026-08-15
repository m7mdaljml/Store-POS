import { computed, ref } from "vue";
import { defineStore } from "pinia";
import type { CartItem } from "../types";

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
  const total = computed(() =>
    Math.max(0, subtotal.value - itemDiscountTotal.value - orderDiscountAmount.value)
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

  /** Order-level discount; type is "fixed" (currency) or "percent" of subtotal. */
  function setOrderDiscount(type: "fixed" | "percent", value: number) {
    orderDiscountType.value = type;
    orderDiscountValue.value = Math.max(0, value);
  }

  function clear() {
    items.value = [];
    orderDiscountType.value = "fixed";
    orderDiscountValue.value = 0;
  }

  return {
    items,
    orderDiscountType,
    orderDiscountValue,
    orderDiscountAmount,
    subtotal,
    itemDiscountTotal,
    total,
    itemCount,
    add,
    remove,
    setQty,
    setDiscount,
    lineTotal,
    setOrderDiscount,
    clear,
  };
});
