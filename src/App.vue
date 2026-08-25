<template>
  <ToastHost />
  <ConfirmHost />
  <div class="app-shell">
    <aside v-if="!isLoginRoute" class="sidebar" :class="{ collapsed }">
      <div class="sidebar-brand">
        <img
          v-if="settings.storeLogo"
          :src="settings.storeLogo"
          class="brand-logo"
          alt=""
        />
        <div v-else class="sidebar-brand-icon">
          <i class="bi bi-shop"></i>
        </div>
        <span v-if="!collapsed" class="fw-semibold fs-6">
          {{ settings.storeName || t("app.storeName") }}
        </span>
      </div>

      <nav class="sidebar-nav">
        <RouterLink
          v-for="item in navItems"
          :key="item.to"
          :to="item.to"
          class="sidebar-link"
          active-class="active"
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
          :aria-label="t('app.toggleSidebar')"
          @click="collapsed = !collapsed"
        >
          <i
            class="bi"
            :class="
              collapsed ? 'bi-chevron-double-right' : 'bi-chevron-double-left'
            "
          ></i>
        </button>

        <h1 class="topbar-title">
          {{ settings.storeName || t("app.storeName") }}
        </h1>

        <div class="topbar-controls d-flex align-items-center gap-3">
          <span class="text-muted small fw-medium">
            <i class="bi bi-clock mx-1"></i>{{ clock }}
          </span>

          <button
            class="btn btn-sm btn-outline-secondary"
            type="button"
            :title="
              theme.isDark
                ? t('app.switchThemeLight')
                : t('app.switchThemeDark')
            "
            @click="theme.toggle()"
          >
            <i class="bi" :class="theme.isDark ? 'bi-sun' : 'bi-moon'"></i>
          </button>

          <button
            class="btn btn-sm btn-outline-secondary"
            type="button"
            :title="t('app.switchLanguage')"
            @click="toggleLocale()"
          >
            <i class="bi bi-translate"></i>
          </button>

          <template v-if="auth.isAuthenticated">
            <span class="role-badge">
              {{
                t(
                  auth.user?.roleName === "Admin"
                    ? "roles.admin"
                    : "roles.cashier",
                )
              }}
            </span>
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
              <i class="bi bi-box-arrow-right mx-1"></i>{{ t("app.logout") }}
            </button>
          </template>

          <RouterLink
            v-else-if="route.name !== 'login'"
            to="/login"
            class="btn btn-sm btn-primary"
          >
            <i class="bi bi-box-arrow-in-right mx-1"></i>{{ t("app.login") }}
          </RouterLink>

          <button
            class="btn btn-sm btn-outline-danger"
            type="button"
            @click="closeApp()"
          >
            <i class="bi bi-x-lg mx-1"></i>Close App
          </button>
        </div>
      </header>

      <main class="content">
        <RouterView />
      </main>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { RouterLink, RouterView, useRoute, useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useAuth } from "./composables/useAuth";
import { useSettingsStore } from "./stores/settings";
import { useCatalogStore } from "./stores/catalog";
import { useThemeStore } from "./stores/theme";
import { seedIfNeeded } from "./lib/seed";
import { startAutoBackup } from "./composables/useAutoBackup";
import ToastHost from "./components/ToastHost.vue";
import ConfirmHost from "./components/ConfirmHost.vue";
import { toggleLocale } from "./i18n";

const route = useRoute();
const router = useRouter();
const { t } = useI18n();
const auth = useAuth();
const settings = useSettingsStore();
const catalog = useCatalogStore();
const theme = useThemeStore();

const collapsed = ref(false);
const clock = ref("");
let timer: number | undefined;

const isLoginRoute = computed(() => route.name === "login");

const navItems = computed(() => {
  const items = [
    {
      to: "/",
      label: t("nav.dashboard"),
      icon: "bi-speedometer2",
      adminOnly: true,
    },
    {
      to: "/checkout",
      label: t("nav.checkout"),
      icon: "bi-basket",
      permission: "sales.checkout",
    },
    {
      to: "/sales",
      label: t("nav.sales"),
      icon: "bi-receipt",
      permission: "sales.void",
    },
    {
      to: "/products",
      label: t("nav.products"),
      icon: "bi-box-seam",
      permission: "inventory.view",
    },
    {
      to: "/stock",
      label: t("nav.stock"),
      icon: "bi-boxes",
      permission: "inventory.view",
    },
    {
      to: "/purchases",
      label: t("nav.purchases"),
      icon: "bi-truck",
      permission: "inventory.view",
    },
    {
      to: "/suppliers",
      label: t("nav.suppliers"),
      icon: "bi-person-lines-fill",
      permission: "expenses.manage",
    },
    {
      to: "/customers",
      label: t("nav.customers"),
      icon: "bi-people",
      adminOnly: true,
    },
    {
      to: "/reports",
      label: t("nav.reports"),
      icon: "bi-graph-up-arrow",
      permission: "reports.view",
    },
    {
      to: "/sessions",
      label: t("nav.sessions"),
      icon: "bi-cash-stack",
      permission: "reports.view",
    },
    {
      to: "/expenses",
      label: t("nav.expenses"),
      icon: "bi-cash-coin",
      permission: "expenses.manage",
    },
    {
      to: "/users",
      label: t("nav.users"),
      icon: "bi-person-gear",
      permission: "users.manage",
    },
    {
      to: "/logs",
      label: t("nav.logs"),
      icon: "bi-journal-text",
      adminOnly: true,
    },
    {
      to: "/settings",
      label: t("nav.settings"),
      icon: "bi-gear",
    },
  ];
  return items.filter((item) => {
    if (item.adminOnly) return auth.role === "Admin";
    if (item.permission) return auth.can(item.permission);
    return true;
  });
});

function tick() {
  clock.value = new Date().toLocaleTimeString();
}

function closeApp() {
  getCurrentWindow().close();
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
  startAutoBackup();
});

watch(
  () => auth.isAuthenticated,
  (authenticated) => {
    if (!authenticated && route.name !== "login") {
      router.push({ name: "login" });
    }
  },
);

onBeforeUnmount(() => {
  if (timer) window.clearInterval(timer);
});
</script>
