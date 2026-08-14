import { computed, ref } from "vue";
import { defineStore } from "pinia";
import type { CartItem } from "../types";

export const useCartStore = defineStore("cart", () => {
  const items = ref<CartItem[]>([]);
  const orderDiscount = ref(0);

  const subtotal = computed(() =>
    items.value.reduce((sum, item) => sum + item.price * item.qty, 0)
  );
  const itemDiscountTotal = computed(() =>
    items.value.reduce((sum, item) => sum + item.discount * item.qty, 0)
  );
  const total = computed(() =>
    Math.max(0, subtotal.value - itemDiscountTotal.value - orderDiscount.value)
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

  function clear() {
    items.value = [];
    orderDiscount.value = 0;
  }

  return {
    items,
    orderDiscount,
    subtotal,
    itemDiscountTotal,
    total,
    itemCount,
    add,
    remove,
    setQty,
    clear,
  };
});
