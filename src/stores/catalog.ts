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
        "SELECT id, sku, barcode, name, description, category_id, cost_price, sell_price, tax_profile_id, unit, stock_qty, reorder_level, image_path, is_active, is_quick FROM products"
      ),
      select<Category>("SELECT id, name FROM categories"),
    ]);
    products.value = p;
    categories.value = c;
    loaded.value = true;
  }

  return { products, categories, loaded, load };
});
