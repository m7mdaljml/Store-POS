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
  },
  {
    path: "/checkout",
    name: "checkout",
    component: () => import("../app/presentation/checkout.vue"),
  },
  {
    path: "/products",
    name: "products",
    component: () => import("../app/presentation/products.vue"),
  },
  {
    path: "/purchases",
    name: "purchases",
    component: () => import("../app/presentation/purchases.vue"),
  },
  {
    path: "/customers",
    name: "customers",
    component: () => import("../app/presentation/customers.vue"),
  },
  {
    path: "/reports",
    name: "reports",
    component: () => import("../app/presentation/reports.vue"),
  },
  {
    path: "/settings",
    name: "settings",
    component: () => import("../app/presentation/settings.vue"),
  },
  {
    path: "/expenses",
    name: "expenses",
    component: () => import("../app/presentation/expenses.vue"),
  },
  {
    path: "/users",
    name: "users",
    component: () => import("../app/presentation/users.vue"),
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
    return { name: "dashboard" };
  }
  return true;
});

export default router;
