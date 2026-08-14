import { ref } from "vue";
import { defineStore } from "pinia";
import { select } from "../lib/db";
import type { Category, Product } from "../types";

export const useCatalogStore = defineStore("catalog", () => {
  const products = ref<Product[]>([]);
  const categories = ref<Category[]>([]);
  const loaded = ref(false);

  async function load() {
    const [p, c] = await Promise.all([
      select<Product>(
        "SELECT id, sku, barcode, name, cost_price, sell_price, stock_qty, category_id FROM products"
      ),
      select<Category>("SELECT id, name FROM categories"),
    ]);
    products.value = p;
    categories.value = c;
    loaded.value = true;
  }

  return { products, categories, loaded, load };
});
