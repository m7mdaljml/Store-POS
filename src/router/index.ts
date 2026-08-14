import { createRouter, createWebHistory } from "vue-router";
import { useAuthStore } from "../stores/auth";

const routes = [
  {
    path: "/login",
    name: "login",
    component: () => import("../app/presentation/login.vue"),
    meta: { public: true },
  },
  {
    path: "/",
    name: "dashboard",
    component: () => import("../app/presentation/dashboard.vue"),
    meta: { adminOnly: true },
  },
  {
    path: "/checkout",
    name: "checkout",
    component: () => import("../app/presentation/checkout.vue"),
    meta: { permission: "sales.checkout" },
  },
  {
    path: "/products",
    name: "products",
    component: () => import("../app/presentation/products.vue"),
    meta: { permission: "inventory.view" },
  },
  {
    path: "/purchases",
    name: "purchases",
    component: () => import("../app/presentation/purchases.vue"),
    meta: { permission: "inventory.view" },
  },
  {
    path: "/customers",
    name: "customers",
    component: () => import("../app/presentation/customers.vue"),
    meta: { adminOnly: true },
  },
  {
    path: "/reports",
    name: "reports",
    component: () => import("../app/presentation/reports.vue"),
    meta: { permission: "reports.view" },
  },
  {
    path: "/settings",
    name: "settings",
    component: () => import("../app/presentation/settings.vue"),
    meta: { permission: "settings.manage" },
  },
  {
    path: "/expenses",
    name: "expenses",
    component: () => import("../app/presentation/expenses.vue"),
    meta: { permission: "expenses.manage" },
  },
  {
    path: "/users",
    name: "users",
    component: () => import("../app/presentation/users.vue"),
    meta: { permission: "users.manage" },
  },
];

const router = createRouter({
  history: createWebHistory(),
  routes,
});

router.beforeEach((to) => {
  const auth = useAuthStore();
  if (!to.meta.public && !auth.isAuthenticated) {
    return { name: "login" };
  }
  if (to.name === "login" && auth.isAuthenticated) {
    return { name: auth.role === "Cashier" ? "checkout" : "dashboard" };
  }
  if (auth.role === "Cashier" && to.name !== "checkout") {
    return { name: "checkout" };
  }
  if (to.meta.permission && !auth.can(to.meta.permission as string)) {
    return { name: auth.role === "Cashier" ? "checkout" : "dashboard" };
  }
  return true;
});

export default router;
