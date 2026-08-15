<template>
  <div>
    <div class="d-flex align-items-center justify-content-between mb-3">
      <h1 class="h4 mb-0">{{ t("users.title") }}</h1>
      <button v-can="'users.manage'" class="btn btn-primary" type="button" @click="openAdd">
        <i class="bi bi-person-plus me-1"></i>{{ t("users.addCashier") }}
      </button>
    </div>

    <div v-if="error" class="alert alert-danger py-2 small" role="alert">
      <i class="bi bi-exclamation-triangle me-1"></i>{{ error }}
    </div>
    <div v-if="notice" class="alert alert-success py-2 small" role="alert">
      <i class="bi bi-check-circle me-1"></i>{{ notice }}
    </div>

    <div class="card">
      <div class="table-responsive">
        <table class="table align-middle mb-0">
          <thead>
            <tr>
              <th>{{ t("users.user") }}</th>
              <th>{{ t("users.role") }}</th>
              <th>{{ t("common.status") }}</th>
              <th>{{ t("users.permissions") }}</th>
              <th class="text-end">{{ t("common.actions") }}</th>
            </tr>
          </thead>
          <tbody>
            <tr v-if="loading">
              <td colspan="5" class="text-center text-muted py-4">{{ t("common.loading") }}</td>
            </tr>
            <tr v-else-if="!users.length">
              <td colspan="5" class="text-center text-muted py-4">{{ t("users.noUsers") }}</td>
            </tr>
            <tr v-for="u in users" :key="u.id">
              <td>
                <div class="fw-semibold">{{ u.fullName }}</div>
                <div class="text-muted small">@{{ u.username }}</div>
              </td>
              <td>
                <span
                  class="badge"
                  :class="u.roleName === 'Admin' ? 'text-bg-warning' : 'text-bg-secondary'"
                >
                  {{ roleLabel(u.roleName) }}
                </span>
              </td>
              <td>
                <span
                  class="badge"
                  :class="u.isActive ? 'text-bg-success' : 'text-bg-danger'"
                >
                  {{ u.isActive ? t("common.active") : t("users.deactivated") }}
                </span>
              </td>
              <td>
                <div class="d-flex flex-wrap gap-1">
                  <span
                    v-for="p in u.permissions"
                    :key="p"
                    class="badge text-bg-light border"
                  >
                    {{ permissionLabel(p) }}
                  </span>
                  <span v-if="!u.permissions.length" class="text-muted small">—</span>
                </div>
              </td>
              <td class="text-end text-nowrap">
                <template v-if="u.username !== auth.user?.username">
                  <button
                    class="btn btn-sm btn-outline-primary me-1"
                    type="button"
                    :title="t('users.editUserTitle')"
                    @click="openEdit(u)"
                  >
                    <i class="bi bi-pencil-square"></i>
                  </button>
                  <button
                    v-if="u.isActive"
                    class="btn btn-sm btn-outline-danger me-1"
                    type="button"
                    :title="t('users.deactivateTitle')"
                    @click="deactivate(u)"
                  >
                    <i class="bi bi-person-dash"></i>
                  </button>
                  <button
                    v-else
                    class="btn btn-sm btn-outline-success me-1"
                    type="button"
                    :title="t('users.reactivateTitle')"
                    @click="activate(u)"
                  >
                    <i class="bi bi-person-check"></i>
                  </button>
                  <button
                    class="btn btn-sm btn-outline-danger"
                    type="button"
                    :title="t('users.deletePermTitle')"
                    @click="remove(u)"
                  >
                    <i class="bi bi-trash"></i>
                  </button>
                </template>
                <span v-else class="text-muted small fst-italic">{{ t("users.you") }}</span>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <div v-if="showAdd" class="modal-backdrop show"></div>
    <div v-if="showAdd" class="modal d-block" tabindex="-1">
      <div class="modal-dialog modal-dialog-centered">
        <div class="modal-content">
          <form @submit.prevent="submitAdd">
            <div class="modal-header">
              <h5 class="modal-title">{{ t("users.addCashierTitle") }}</h5>
              <button type="button" class="btn-close" @click="showAdd = false"></button>
            </div>
            <div class="modal-body">
              <div class="mb-3">
                <label class="form-label" for="new-fullname">{{ t("users.fullName") }}</label>
                <input id="new-fullname" v-model="addForm.fullName" class="form-control" type="text" />
              </div>
              <div class="mb-3">
                <label class="form-label" for="new-username">{{ t("common.username") }}</label>
                <input id="new-username" v-model="addForm.username" class="form-control" type="text" autocomplete="off" />
              </div>
              <div class="mb-3">
                <label class="form-label" for="new-password">{{ t("common.password") }}</label>
                <input id="new-password" v-model="addForm.password" class="form-control" type="password" autocomplete="new-password" />
              </div>
              <div class="mb-0">
                <label class="form-label" for="new-role">{{ t("users.role") }}</label>
                <select id="new-role" v-model="addForm.roleId" class="form-select">
                  <option v-for="r in roles" :key="r.id" :value="r.id">{{ roleLabel(r.name) }}</option>
                </select>
              </div>
            </div>
            <div class="modal-footer">
              <button type="button" class="btn btn-outline-secondary" @click="showAdd = false">
                {{ t("common.cancel") }}
              </button>
              <button type="submit" class="btn btn-primary" :disabled="adding">
                <span v-if="adding" class="spinner-border spinner-border-sm me-2"></span>{{ t("users.create") }}
              </button>
            </div>
          </form>
        </div>
      </div>
    </div>

    <div v-if="editTarget" class="modal-backdrop show"></div>
    <div v-if="editTarget" class="modal d-block" tabindex="-1">
      <div class="modal-dialog modal-dialog-centered">
        <div class="modal-content">
          <form @submit.prevent="saveUser">
            <div class="modal-header">
              <h5 class="modal-title">{{ t("users.editTitle", { name: editTarget.fullName }) }}</h5>
              <button type="button" class="btn-close" @click="editTarget = null"></button>
            </div>
            <div class="modal-body">
              <div class="mb-3">
                <label class="form-label" for="edit-fullname">{{ t("users.fullName") }}</label>
                <input id="edit-fullname" v-model="editForm.fullName" class="form-control" type="text" />
              </div>
              <div class="mb-3">
                <label class="form-label" for="edit-username">{{ t("common.username") }}</label>
                <input id="edit-username" v-model="editForm.username" class="form-control" type="text" autocomplete="off" />
              </div>
              <div class="mb-3">
                <label class="form-label" for="edit-password">
                  {{ t("common.password") }}
                  <span class="text-muted fw-normal">{{ t("users.passwordKeep") }}</span>
                </label>
                <input id="edit-password" v-model="editForm.password" class="form-control" type="password" autocomplete="new-password" />
              </div>
              <div class="mb-3">
                <label class="form-label" for="edit-role">{{ t("users.role") }}</label>
                <select id="edit-role" v-model="editForm.roleId" class="form-select">
                  <option v-for="r in roles" :key="r.id" :value="r.id">{{ roleLabel(r.name) }}</option>
                </select>
              </div>
              <hr />
              <div class="mb-2 fw-semibold small text-muted">{{ t("users.permissions") }}</div>
              <div
                v-for="p in permissions"
                :key="p"
                class="d-flex justify-content-between align-items-center py-1"
              >
                <label class="small" :for="'edit-perm-' + p">{{ permissionLabel(p) }}</label>
                <div class="form-check form-switch m-0">
                  <input
                    class="form-check-input"
                    type="checkbox"
                    role="switch"
                    :id="'edit-perm-' + p"
                    :checked="editSelection.includes(p)"
                    @change="togglePerm(p, ($event.target as HTMLInputElement).checked)"
                  />
                </div>
              </div>
            </div>
            <div class="modal-footer">
              <button type="button" class="btn btn-outline-secondary" @click="editTarget = null">
                {{ t("common.cancel") }}
              </button>
              <button type="submit" class="btn btn-primary" :disabled="saving">
                <span v-if="saving" class="spinner-border spinner-border-sm me-2"></span>{{ t("common.save") }}
              </button>
            </div>
          </form>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useI18n } from "vue-i18n";
import { useAuth } from "../../composables/useAuth";
import { i18n } from "../../i18n";
import { permissionLabel as rawPermissionLabel } from "../../lib/permissions";
import type { Role, UserRecord } from "../../types";

const auth = useAuth();
const { t } = useI18n();

function permissionLabel(code: string): string {
  const messages = i18n.global.getLocaleMessage(i18n.global.locale.value) as {
    permissions?: Record<string, string>;
  };
  return messages?.permissions?.[code] ?? rawPermissionLabel(code);
}

function roleLabel(name: string): string {
  return name.toLowerCase() === "admin" ? t("roles.admin") : t("roles.cashier");
}

const users = ref<UserRecord[]>([]);
const roles = ref<Role[]>([]);
const permissions = ref<string[]>([]);
const loading = ref(false);
const error = ref("");
const notice = ref("");

const showAdd = ref(false);
const adding = ref(false);
const addForm = ref({ username: "", fullName: "", password: "", roleId: 0 });

const editTarget = ref<UserRecord | null>(null);
const saving = ref(false);
const editSelection = ref<string[]>([]);
const editForm = ref({ username: "", fullName: "", password: "", roleId: 0 });

function cashierRoleId() {
  return roles.value.find((r) => r.name === "Cashier")?.id ?? roles.value[0]?.id ?? 0;
}

async function load() {
  loading.value = true;
  error.value = "";
  try {
    const [u, r, p] = await Promise.all([
      invoke<UserRecord[]>("list_users"),
      invoke<Role[]>("list_roles"),
      invoke<string[]>("list_permissions"),
    ]);
    users.value = u;
    roles.value = r;
    permissions.value = p;
    if (!addForm.value.roleId) addForm.value.roleId = cashierRoleId();
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

function openAdd() {
  error.value = "";
  addForm.value = { username: "", fullName: "", password: "", roleId: cashierRoleId() };
  showAdd.value = true;
}

async function submitAdd() {
  error.value = "";
  if (!addForm.value.username || !addForm.value.password || !addForm.value.fullName) {
    error.value = t("users.allFieldsRequired");
    return;
  }
  adding.value = true;
  try {
    await invoke<number>("create_user", {
      username: addForm.value.username.trim(),
      password: addForm.value.password,
      fullName: addForm.value.fullName.trim(),
      roleId: addForm.value.roleId,
    });
    showAdd.value = false;
    notice.value = t("users.cashierCreated");
    await load();
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    adding.value = false;
  }
}

function openEdit(u: UserRecord) {
  error.value = "";
  editTarget.value = u;
  editSelection.value = [...u.permissions];
  editForm.value = {
    username: u.username,
    fullName: u.fullName,
    password: "",
    roleId: u.roleId,
  };
}

function togglePerm(code: string, on: boolean) {
  if (on) {
    if (!editSelection.value.includes(code)) editSelection.value.push(code);
  } else {
    editSelection.value = editSelection.value.filter((c) => c !== code);
  }
}

async function saveUser() {
  if (!editTarget.value) return;
  error.value = "";
  if (!editForm.value.username || !editForm.value.fullName) {
    error.value = t("users.usernameNameRequired");
    return;
  }
  saving.value = true;
  try {
    await invoke("update_user", {
      userId: editTarget.value.id,
      username: editForm.value.username.trim(),
      fullName: editForm.value.fullName.trim(),
      password: editForm.value.password || null,
      roleId: editForm.value.roleId,
    });
    await invoke<string[]>("update_user_permissions", {
      userId: editTarget.value.id,
      permissionCodes: editSelection.value,
    });
    editTarget.value = null;
    notice.value = t("users.userUpdated");
    await load();
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    saving.value = false;
  }
}

async function deactivate(u: UserRecord) {
  error.value = "";
  if (!window.confirm(t("users.deactivateConfirm", { name: u.fullName }))) return;
  try {
    await invoke("delete_user", { userId: u.id });
    notice.value = t("users.deactivated2", { name: u.fullName });
    await load();
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  }
}

async function activate(u: UserRecord) {
  error.value = "";
  try {
    await invoke("set_user_active", { userId: u.id, active: true });
    notice.value = t("users.reactivated", { name: u.fullName });
    await load();
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  }
}

async function remove(u: UserRecord) {
  error.value = "";
  const msg = t("users.deleteConfirm", { name: u.fullName, username: u.username });
  if (!window.confirm(msg)) return;
  try {
    await invoke("remove_user", { userId: u.id });
    notice.value = t("users.deleted", { name: u.fullName });
    await load();
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  }
}

onMounted(load);
</script>
