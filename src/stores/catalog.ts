import { ref } from "vue";
import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import type { Category, Product } from "../types";

export const useCatalogStore = defineStore("catalog", () => {
  const products = ref<Product[]>([]);
  const categories = ref<Category[]>([]);
  const loaded = ref(false);

  async function load() {
    const [p, c] = await Promise.all([
      invoke<Product[]>("list_products"),
      invoke<Category[]>("list_categories"),
    ]);
    products.value = p;
    categories.value = c;
    loaded.value = true;
  }

  return { products, categories, loaded, load };
});
