<template>
  <div
    class="d-flex align-items-center justify-content-center"
    style="min-height: calc(100vh - 64px)"
  >
    <div class="card shadow-sm" style="width: 380px">
      <div class="card-body p-4">
        <div class="text-center mb-4">
          <div
            class="d-inline-flex align-items-center justify-content-center rounded-3 text-white mb-3"
            style="
              width: 56px;
              height: 56px;
              font-size: 1.6rem;
              background: linear-gradient(135deg, var(--pos-accent), #22d3ee);
            "
          >
            <i class="bi bi-shop"></i>
          </div>
          <h1 class="h5 mb-1">{{ settings.storeName || t("app.storeName") }}</h1>
          <div class="text-muted small">{{ t("login.subtitle") }}</div>
        </div>

        <form @submit.prevent="submit">
          <div v-if="error" class="alert alert-danger py-2 small" role="alert">
            <i class="bi bi-exclamation-triangle me-1"></i>{{ error }}
          </div>

          <div class="mb-3">
            <label for="login-username" class="form-label">
              {{ t("common.username") }}
            </label>
            <input
              id="login-username"
              v-model="username"
              type="text"
              class="form-control"
              autocomplete="username"
              autofocus
            />
          </div>

          <div class="mb-4">
            <label for="login-password" class="form-label">
              {{ t("common.password") }}
            </label>
            <input
              id="login-password"
              v-model="password"
              type="password"
              class="form-control"
              autocomplete="current-password"
            />
          </div>

          <button
            class="btn btn-primary w-100"
            type="submit"
            :disabled="submitting"
          >
            <span
              v-if="submitting"
              class="spinner-border spinner-border-sm me-2"
            ></span>
            {{ t("login.signIn") }}
          </button>
        </form>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import { useAuth } from "../../composables/useAuth";
import { useSettingsStore } from "../../stores/settings";

const router = useRouter();
const { t } = useI18n();
const auth = useAuth();
const settings = useSettingsStore();

const username = ref("");
const password = ref("");
const error = ref("");
const submitting = ref(false);

async function submit() {
  error.value = "";
  if (!username.value || !password.value) {
    error.value = t("login.required");
    return;
  }
  submitting.value = true;
  try {
    await auth.login(username.value, password.value);
    router.push(auth.user?.roleName === "Cashier" ? "/checkout" : "/");
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    submitting.value = false;
  }
}
</script>
