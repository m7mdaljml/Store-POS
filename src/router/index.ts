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
    path: "/sales",
    name: "sales",
    component: () => import("../app/presentation/sales.vue"),
    meta: { permission: "sales.void" },
  },
  {
    path: "/products",
    name: "products",
    component: () => import("../app/presentation/products.vue"),
    meta: { permission: "inventory.view" },
  },
  {
    path: "/stock",
    name: "stock",
    component: () => import("../app/presentation/stock.vue"),
    meta: { permission: "inventory.view" },
  },
  {
    path: "/purchases",
    name: "purchases",
    component: () => import("../app/presentation/purchases.vue"),
    meta: { permission: "inventory.view" },
  },
  {
    path: "/suppliers",
    name: "suppliers",
    component: () => import("../app/presentation/suppliers.vue"),
    meta: { permission: "expenses.manage" },
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
    path: "/sessions",
    name: "sessions",
    component: () => import("../app/presentation/sessions.vue"),
    meta: { permission: "reports.view" },
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
    meta: { permission: "expenses.manage" },
  },
  {
    path: "/users",
    name: "users",
    component: () => import("../app/presentation/users.vue"),
    meta: { permission: "users.manage" },
  },
  {
    path: "/logs",
    name: "logs",
    component: () => import("../app/presentation/logs.vue"),
    meta: { adminOnly: true },
  },
];

const router = createRouter({
  history: createWebHistory(),
  routes,
});

interface RouteMetaLike {
  public?: boolean;
  adminOnly?: boolean;
  permission?: string;
}

function isAdminRole(auth: ReturnType<typeof useAuthStore>): boolean {
  return auth.role === "Admin";
}

function canAccess(
  auth: ReturnType<typeof useAuthStore>,
  meta: RouteMetaLike,
): boolean {
  if (isAdminRole(auth)) return true;
  if (meta.adminOnly) return false;
  return !meta.permission || auth.can(meta.permission);
}

function firstAllowedName(auth: ReturnType<typeof useAuthStore>): string | null {
  for (const r of routes) {
    const meta = (r.meta ?? {}) as RouteMetaLike;
    if (meta.public) continue;
    if (canAccess(auth, meta)) return r.name as string;
  }
  return null;
}

router.beforeEach((to) => {
  const auth = useAuthStore();
  if (!to.meta.public && !auth.isAuthenticated) {
    return { name: "login" };
  }
  if (to.name === "login" && auth.isAuthenticated) {
    const home = firstAllowedName(auth);
    return home ? { name: home } : true;
  }
  if (
    canAccess(auth, {
      public: to.meta.public as boolean | undefined,
      adminOnly: to.meta.adminOnly as boolean | undefined,
      permission: to.meta.permission as string | undefined,
    })
  ) {
    return true;
  }
  const fallback = firstAllowedName(auth);
  return fallback ? { name: fallback } : { name: "login" };
});

export default router;
