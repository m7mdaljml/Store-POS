<template>
  <div class="login-wrap">
    <div class="card shadow-sm login-card">
      <div class="card-body">
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
            <i class="bi bi-key"></i>
          </div>
          <h1 class="h5 mb-1">{{ t("changePassword.title") }}</h1>
          <div class="text-muted small">{{ t("changePassword.subtitle") }}</div>
        </div>

        <form novalidate @submit.prevent="submit">
          <div v-if="error" class="alert alert-danger py-2 small" role="alert">
            <i class="bi bi-exclamation-triangle mx-1"></i>{{ error }}
          </div>
          <div class="alert alert-info py-2 small" role="alert">
            <i class="bi bi-shield-lock mx-1"></i>{{ t("changePassword.note") }}
          </div>

          <div class="mb-3">
            <label for="cp-new" class="form-label">
              {{ t("changePassword.newPassword") }}
            </label>
            <input
              id="cp-new"
              v-model="newPassword"
              :type="show ? 'text' : 'password'"
              class="form-control"
              :class="{ 'is-invalid': !!fieldErrors.newPassword }"
              autocomplete="new-password"
              autofocus
              @input="clearFieldError('newPassword')"
            />
            <div v-if="fieldErrors.newPassword" class="invalid-feedback d-block">
              {{ fieldErrors.newPassword }}
            </div>
          </div>

          <div class="mb-4">
            <label for="cp-confirm" class="form-label">
              {{ t("changePassword.confirm") }}
            </label>
            <input
              id="cp-confirm"
              v-model="confirm"
              :type="show ? 'text' : 'password'"
              class="form-control"
              :class="{ 'is-invalid': !!fieldErrors.confirm }"
              autocomplete="new-password"
              @input="clearFieldError('confirm')"
            />
            <div v-if="fieldErrors.confirm" class="invalid-feedback d-block">
              {{ fieldErrors.confirm }}
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
            {{ t("changePassword.save") }}
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
import { useToast } from "../../composables/useToast";

const router = useRouter();
const { t } = useI18n();
const auth = useAuth();
const toast = useToast();

const newPassword = ref("");
const confirm = ref("");
const show = ref(false);
const error = ref("");
const submitting = ref(false);
const fieldErrors = ref<{ newPassword?: string; confirm?: string }>({});

function clearFieldError(field: "newPassword" | "confirm") {
  fieldErrors.value[field] = undefined;
  error.value = "";
}

async function submit() {
  error.value = "";
  fieldErrors.value = {};

  const errors: typeof fieldErrors.value = {};
  if (newPassword.value.length < 4) {
    errors.newPassword = t("changePassword.tooShort");
  }
  if (newPassword.value !== confirm.value) {
    errors.confirm = t("changePassword.mismatch");
  }
  if (errors.newPassword || errors.confirm) {
    fieldErrors.value = errors;
    return;
  }

  submitting.value = true;
  try {
    await auth.setOwnPassword(newPassword.value);
    toast.success(t("changePassword.saved"));
    await router.push(auth.user?.roleName === "Cashier" ? "/checkout" : "/");
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    submitting.value = false;
  }
}
</script>

<style scoped>
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
