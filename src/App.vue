<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from "vue";
import { RouterLink, RouterView, useRoute, useRouter } from "vue-router";
import { useAuthStore } from "./stores/auth";
import { useSettingsStore } from "./stores/settings";
import { useCatalogStore } from "./stores/catalog";
import { useThemeStore } from "./stores/theme";
import { seedIfNeeded } from "./lib/seed";

const route = useRoute();
const router = useRouter();
const auth = useAuthStore();
const settings = useSettingsStore();
const catalog = useCatalogStore();
const theme = useThemeStore();

const collapsed = ref(false);
const clock = ref("");
let timer: number | undefined;

const navItems = [
  { to: "/", label: "Dashboard", icon: "bi-speedometer2" },
  { to: "/checkout", label: "Checkout", icon: "bi-basket" },
  { to: "/products", label: "Products", icon: "bi-box-seam" },
  { to: "/purchases", label: "Purchases", icon: "bi-truck" },
  { to: "/customers", label: "Customers", icon: "bi-people" },
  { to: "/reports", label: "Reports", icon: "bi-graph-up-arrow" },
  { to: "/expenses", label: "Expenses", icon: "bi-cash-coin" },
  { to: "/users", label: "Users", icon: "bi-person-gear" },
  { to: "/settings", label: "Settings", icon: "bi-gear" },
];

function tick() {
  clock.value = new Date().toLocaleTimeString();
}

onMounted(async () => {
  tick();
  timer = window.setInterval(tick, 1000);
  await auth.verifySession();
  try {
    await seedIfNeeded();
  } catch (e) {
    console.error("Seeding failed:", e);
  }
  await Promise.allSettled([settings.load(), catalog.load()]);
});

watch(
  () => auth.isAuthenticated,
  (authenticated) => {
    if (!authenticated && route.name !== "login") {
      router.push({ name: "login" });
    }
  }
);

onBeforeUnmount(() => {
  if (timer) window.clearInterval(timer);
});
</script>

<template>
  <div class="app-shell">
    <aside class="sidebar" :class="{ collapsed }">
      <div class="sidebar-brand">
        <div class="sidebar-brand-icon">
          <i class="bi bi-shop"></i>
        </div>
        <span v-if="!collapsed" class="fw-semibold fs-6">
          {{ settings.storeName || "Store POS" }}
        </span>
      </div>

      <nav class="sidebar-nav">
        <RouterLink
          v-for="item in navItems"
          :key="item.to"
          :to="item.to"
          class="sidebar-link"
          :title="item.label"
        >
          <i class="bi sidebar-link-icon" :class="item.icon"></i>
          <span v-if="!collapsed">{{ item.label }}</span>
        </RouterLink>
      </nav>
    </aside>

    <div class="main-area">
      <header class="topbar">
        <button
          v-if="auth.isAuthenticated"
          class="sidebar-toggle"
          type="button"
          aria-label="Toggle sidebar"
          @click="collapsed = !collapsed"
        >
          <i
            class="bi"
            :class="
              collapsed ? 'bi-chevron-double-right' : 'bi-chevron-double-left'
            "
          ></i>
        </button>

        <h1 class="topbar-title">Store POS</h1>

        <div class="ms-auto d-flex align-items-center gap-3">
          <span class="text-muted small fw-medium">
            <i class="bi bi-clock me-1"></i>{{ clock }}
          </span>

          <button
            class="btn btn-sm btn-outline-secondary"
            type="button"
            :title="
              theme.isDark ? 'Switch to light theme' : 'Switch to dark theme'
            "
            @click="theme.toggle()"
          >
            <i class="bi" :class="theme.isDark ? 'bi-sun' : 'bi-moon'"></i>
          </button>

          <template v-if="auth.isAuthenticated">
            <span class="role-badge">{{ auth.user?.roleName }}</span>
            <div class="d-flex align-items-center gap-2">
              <div
                class="d-flex align-items-center justify-content-center rounded-circle text-bg-primary"
                style="width: 32px; height: 32px"
              >
                {{ auth.user?.fullName.charAt(0).toUpperCase() }}
              </div>
              <div class="lh-1">
                <div class="fw-semibold small">{{ auth.user?.fullName }}</div>
                <div class="text-muted" style="font-size: 0.72rem">
                  @{{ auth.user?.username }}
                </div>
              </div>
            </div>
            <button
              class="btn btn-sm btn-outline-secondary"
              type="button"
              @click="auth.logout()"
            >
              <i class="bi bi-box-arrow-right me-1"></i>Logout
            </button>
          </template>

          <RouterLink
            v-else-if="route.name !== 'login'"
            to="/login"
            class="btn btn-sm btn-primary"
          >
            <i class="bi bi-box-arrow-in-right me-1"></i>Login
          </RouterLink>
        </div>
      </header>

      <main class="content">
        <RouterView />
      </main>
    </div>
  </div>
</template>
