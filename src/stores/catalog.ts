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
        "SELECT p.id, p.sku, p.barcode, p.name, p.description, p.category_id, p.cost_price, p.sell_price, p.tax_profile_id, COALESCE(tp.rate, 0) AS tax_rate, p.unit, p.stock_qty, p.reorder_level, p.image_path, p.is_active, p.is_quick FROM products p LEFT JOIN tax_profiles tp ON tp.id = p.tax_profile_id"
      ),
      select<Category>("SELECT id, name FROM categories"),
    ]);
    products.value = p;
    categories.value = c;
    loaded.value = true;
  }

  return { products, categories, loaded, load };
});
