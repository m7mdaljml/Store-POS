<template>
  <div class="login-wrap">
    <div class="card shadow-sm login-card">
      <div class="card-body">
        <div class="text-center mb-4">
          <img
            v-if="settings.storeLogo"
            :src="settings.storeLogo"
            class="login-logo mb-3"
            alt=""
          />
          <div
            v-else
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
          <h1 class="h5 mb-1">
            {{ settings.storeName || t("app.storeName") }}
          </h1>
          <div class="text-muted small">
            {{
              setupMode
                ? t("setup.subtitle")
                : mustChangeAfterLogin
                  ? t("login.mustChangeSubtitle")
                  : t("login.subtitle")
            }}
          </div>
        </div>

        <form novalidate @submit.prevent="submit">
          <div v-if="error" class="alert alert-danger py-2 small" role="alert">
            <i class="bi bi-exclamation-triangle mx-1"></i>{{ error }}
          </div>

          <div v-if="setupMode" class="mb-3">
            <label for="login-fullname" class="form-label">
              {{ t("setup.fullName") }}
            </label>
            <input
              id="login-fullname"
              v-model="fullName"
              type="text"
              class="form-control"
              :class="{ 'is-invalid': !!fieldErrors.fullName }"
              autocomplete="name"
              @input="clearFieldError('fullName')"
            />
            <div v-if="fieldErrors.fullName" class="invalid-feedback d-block">
              {{ fieldErrors.fullName }}
            </div>
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
              :class="{ 'is-invalid': !!fieldErrors.username || authFailed }"
              autocomplete="username"
              autofocus
              @input="clearFieldError('username')"
            />
            <div v-if="fieldErrors.username" class="invalid-feedback d-block">
              {{ fieldErrors.username }}
            </div>
          </div>

          <div class="mb-4">
            <label for="login-password" class="form-label">
              {{
                setupMode
                  ? t("setup.choosePassword")
                  : t("common.password")
              }}
            </label>
            <div class="input-group">
              <input
                id="login-password"
                v-model="password"
                :type="showPassword ? 'text' : 'password'"
                class="form-control"
                :class="{ 'is-invalid': !!fieldErrors.password || authFailed }"
                :autocomplete="setupMode ? 'new-password' : 'current-password'"
                @input="clearFieldError('password')"
              />
              <button
                class="btn btn-outline-secondary"
                type="button"
                tabindex="-1"
                :aria-label="
                  showPassword
                    ? t('login.hidePassword')
                    : t('login.showPassword')
                "
                @click="showPassword = !showPassword"
              >
                <i
                  class="bi"
                  :class="showPassword ? 'bi-eye-slash' : 'bi-eye'"
                ></i>
              </button>
            </div>
            <div v-if="fieldErrors.password" class="invalid-feedback d-block">
              {{ fieldErrors.password }}
            </div>
          </div>

          <button
            class="btn btn-primary w-100"
            type="submit"
            :disabled="submitting"
          >
            <span
              v-if="submitting"
              class="spinner-border spinner-border-sm mx-2"
            ></span>
            {{
              setupMode ? t("setup.createAdmin") : t("login.signIn")
            }}
          </button>
        </form>

        <div v-if="setupMode" class="alert alert-info py-2 small mt-3 mb-0">
          <i class="bi bi-shield-lock mx-1"></i>{{ t("setup.securityNote") }}
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { useAuth } from "../../composables/useAuth";
import { useSettingsStore } from "../../stores/settings";

const router = useRouter();
const { t } = useI18n();
const auth = useAuth();
const settings = useSettingsStore();

const setupMode = ref(false);
const mustChangeAfterLogin = ref(false);
const username = ref("");
const fullName = ref("");
const password = ref("");
const error = ref("");
const submitting = ref(false);
const showPassword = ref(false);
const authFailed = ref(false);
const fieldErrors = ref<{
  username?: string;
  password?: string;
  fullName?: string;
}>({});

function clearFieldError(field: "username" | "password" | "fullName") {
  fieldErrors.value[field] = undefined;
  authFailed.value = false;
}

async function init() {
  try {
    setupMode.value = await invoke<boolean>("needs_setup");
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  }
}

async function submit() {
  error.value = "";
  authFailed.value = false;
  fieldErrors.value = {};

  const errors: typeof fieldErrors.value = {};
  if (setupMode.value && !fullName.value.trim()) {
    errors.fullName = t("setup.requiredFullName");
  }
  if (!username.value.trim()) {
    errors.username = t("login.requiredUsername");
  }
  if (!password.value) {
    errors.password = t("login.requiredPassword");
  }
  if (errors.username || errors.password || errors.fullName) {
    fieldErrors.value = errors;
    return;
  }

  submitting.value = true;
  try {
    if (setupMode.value) {
      await invoke("setup_admin", {
        username: username.value.trim(),
        password: password.value,
        fullName: fullName.value.trim(),
      });
      setupMode.value = false;
      // Now sign in as the new administrator.
      await auth.login(username.value.trim(), password.value);
    } else {
      await auth.login(username.value.trim(), password.value);
    }
    if (auth.user?.mustChangePassword) {
      mustChangeAfterLogin.value = true;
      await router.push({ name: "change-password" });
    } else {
      await router.push(
        auth.user?.roleName === "Cashier" ? "/checkout" : "/",
      );
    }
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
    authFailed.value = !setupMode.value;
  } finally {
    submitting.value = false;
  }
}

init();
</script>

<style scoped>
.login-logo {
  width: 64px;
  height: 64px;
  object-fit: contain;
}

/* Card sits slightly above center (~46% of viewport height) */
.login-wrap {
  min-height: calc(100vh - 64px - var(--pos-space-xl) * 2);
  display: flex;
  align-items: center;
  justify-content: center;
  padding-bottom: 9vh;
}

.login-card {
  width: 400px;
  max-width: 100%;
  border-radius: var(--pos-radius-lg);
}

.login-card .card-body {
  padding: var(--pos-space-2xl);
}
</style>
