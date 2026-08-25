<template>
  <div>
    <div class="d-flex align-items-center justify-content-between mb-3">
      <h1 class="h4 mb-0">{{ t("users.title") }}</h1>
      <button
        v-can="'users.manage'"
        class="btn btn-primary"
        type="button"
        @click="openAdd"
      >
        <i class="bi bi-person-plus mx-1"></i>{{ t("users.addCashier") }}
      </button>
    </div>



    <div class="card">
      <div class="p-2 border-bottom">
        <input
          v-model="search"
          class="form-control form-control-sm"
          type="search"
          :placeholder="t('users.searchPlaceholder')"
        />
      </div>
      <div class="table-responsive">
        <table class="table align-middle mb-0">
          <thead v-if="users.length">
            <tr>
              <th>{{ t("users.user") }}</th>
              <th>{{ t("users.role") }}</th>
              <th>{{ t("common.status") }}</th>
              <th>{{ t("users.permissions") }}</th>
              <th class="text-end">{{ t("common.actions") }}</th>
            </tr>
          </thead>
          <tbody v-if="loading">
            <tr>
              <td colspan="5" class="text-center text-muted py-4">
                {{ t("common.loading") }}
              </td>
            </tr>
          </tbody>
          <tbody v-else-if="!users.length">
            <tr>
              <td colspan="5" class="p-0 border-0">
                <EmptyState :image="emptyUsers" :message="t('users.noUsers')" />
              </td>
            </tr>
          </tbody>
          <tbody v-for="u in users" :key="u.id">
            <tr>
              <td>
                <div class="fw-semibold">{{ u.fullName }}</div>
                <div class="text-muted small">@{{ u.username }}</div>
              </td>
              <td>
                <span
                  class="badge"
                  :class="
                    u.roleName === 'Admin'
                      ? 'text-bg-warning'
                      : 'text-bg-secondary'
                  "
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
                  <span v-if="!u.permissions.length" class="text-muted small"
                    >—</span
                  >
                </div>
              </td>
              <td class="text-end text-nowrap">
                <template v-if="u.username !== auth.user?.username">
                  <button
                    class="btn btn-sm btn-outline-primary mx-1"
                    type="button"
                    :title="t('users.editUserTitle')"
                    @click="openEdit(u)"
                  >
                    <i class="bi bi-pencil-square"></i>
                  </button>
                  <button
                    v-if="u.isActive"
                    class="btn btn-sm btn-outline-danger mx-1"
                    type="button"
                    :title="t('users.deactivateTitle')"
                    @click="deactivate(u)"
                  >
                    <i class="bi bi-person-dash"></i>
                  </button>
                  <button
                    v-else
                    class="btn btn-sm btn-outline-success mx-1"
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
                <span v-else class="text-muted small fst-italic">{{
                  t("users.you")
                }}</span>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
      <Paginator
        :page="page"
        :page-size="size"
        :total-items="totalItems"
        :disabled="loading"
        @update:page="goToPage"
      />
    </div>

    <div v-if="showAdd" class="modal-backdrop show"></div>
    <div v-if="showAdd" class="modal d-block" tabindex="-1">
      <div class="modal-dialog modal-dialog-centered">
        <div class="modal-content">
          <form @submit.prevent="submitAdd">
            <div class="modal-header">
              <h5 class="modal-title">{{ t("users.addCashierTitle") }}</h5>
              <button
                type="button"
                class="btn-close"
                @click="showAdd = false"
              ></button>
            </div>
            <div class="modal-body">
              <div class="mb-3">
                <label class="form-label" for="new-fullname">{{
                  t("users.fullName")
                }}</label>
                <input
                  id="new-fullname"
                  v-model="addForm.fullName"
                  class="form-control"
                  :class="{ 'is-invalid': addErrors.fullName }"
                  type="text"
                  @input="clearFieldError(addErrors, 'fullName')"
                />
                <div class="invalid-feedback">{{ addErrors.fullName }}</div>
              </div>
              <div class="mb-3">
                <label class="form-label" for="new-username">{{
                  t("common.username")
                }}</label>
                <input
                  id="new-username"
                  v-model="addForm.username"
                  class="form-control"
                  :class="{ 'is-invalid': addErrors.username }"
                  type="text"
                  autocomplete="off"
                  @input="clearFieldError(addErrors, 'username')"
                />
                <div class="invalid-feedback">{{ addErrors.username }}</div>
              </div>
              <div class="mb-3">
                <label class="form-label" for="new-password">{{
                  t("common.password")
                }}</label>
                <input
                  id="new-password"
                  v-model="addForm.password"
                  class="form-control"
                  :class="{ 'is-invalid': addErrors.password }"
                  type="password"
                  autocomplete="new-password"
                  @input="clearFieldError(addErrors, 'password')"
                />
                <div class="invalid-feedback">{{ addErrors.password }}</div>
              </div>
              <div class="mb-0">
                <label class="form-label" for="new-role">{{
                  t("users.role")
                }}</label>
                <AppSelect
                  id="new-role"
                  v-model="addForm.roleId"
                  :items="roles"
                  :option-label="(r) => roleLabel(r.name)"
                  :option-value="(r) => r.id"
                  :class="{ 'is-invalid': addErrors.roleId }"
                />
                <div class="invalid-feedback">{{ addErrors.roleId }}</div>
              </div>
              <div
                class="d-flex justify-content-end gap-2 mt-3 pt-3 border-top"
              >
                <button
                  type="button"
                  class="btn btn-outline-secondary"
                  @click="showAdd = false"
                >
                  {{ t("common.cancel") }}
                </button>
                <AsyncButton
                  type="submit"
                  :loading="adding"
                  :disabled="!canAdd"
                >
                  {{ t("users.create") }}
                </AsyncButton>
              </div>
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
              <h5 class="modal-title">
                {{ t("users.editTitle", { name: editTarget.fullName }) }}
              </h5>
              <button
                type="button"
                class="btn-close"
                @click="editTarget = null"
              ></button>
            </div>
            <div class="modal-body">
              <div class="mb-3">
                <label class="form-label" for="edit-fullname">{{
                  t("users.fullName")
                }}</label>
                <input
                  id="edit-fullname"
                  v-model="editForm.fullName"
                  class="form-control"
                  :class="{ 'is-invalid': editErrors.fullName }"
                  type="text"
                  @input="clearFieldError(editErrors, 'fullName')"
                />
                <div class="invalid-feedback">{{ editErrors.fullName }}</div>
              </div>
              <div class="mb-3">
                <label class="form-label" for="edit-username">{{
                  t("common.username")
                }}</label>
                <input
                  id="edit-username"
                  v-model="editForm.username"
                  class="form-control"
                  :class="{ 'is-invalid': editErrors.username }"
                  type="text"
                  autocomplete="off"
                  @input="clearFieldError(editErrors, 'username')"
                />
                <div class="invalid-feedback">{{ editErrors.username }}</div>
              </div>
              <div class="mb-3">
                <label class="form-label" for="edit-password">
                  {{ t("common.password") }}
                  <span class="text-muted fw-normal">{{
                    t("users.passwordKeep")
                  }}</span>
                </label>
                <input
                  id="edit-password"
                  v-model="editForm.password"
                  class="form-control"
                  type="password"
                  autocomplete="new-password"
                />
              </div>
              <div class="mb-3">
                <label class="form-label" for="edit-role">{{
                  t("users.role")
                }}</label>
                <AppSelect
                  id="edit-role"
                  v-model="editForm.roleId"
                  :items="roles"
                  :option-label="(r) => roleLabel(r.name)"
                  :option-value="(r) => r.id"
                />
              </div>
              <hr />
              <div class="mb-2 fw-semibold small text-muted">
                {{ t("users.permissions") }}
              </div>
              <div
                v-for="p in permissions"
                :key="p"
                class="d-flex justify-content-between align-items-center py-1"
              >
                <label class="small" :for="'edit-perm-' + p">{{
                  permissionLabel(p)
                }}</label>
                <div class="form-check form-switch m-0">
                  <input
                    class="form-check-input"
                    type="checkbox"
                    role="switch"
                    :id="'edit-perm-' + p"
                    :checked="editSelection.includes(p)"
                    @change="
                      togglePerm(p, ($event.target as HTMLInputElement).checked)
                    "
                  />
                </div>
              </div>
              <div
                class="d-flex justify-content-end gap-2 mt-3 pt-3 border-top"
              >
                <button
                  type="button"
                  class="btn btn-outline-secondary"
                  @click="editTarget = null"
                >
                  {{ t("common.cancel") }}
                </button>
                <AsyncButton
                  type="submit"
                  :loading="saving"
                  :disabled="!canSaveEdit"
                >
                  {{ t("common.save") }}
                </AsyncButton>
              </div>
            </div>
          </form>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useI18n } from "vue-i18n";
import EmptyState from "../../components/EmptyState.vue";
import AsyncButton from "../../components/AsyncButton.vue";
import Paginator from "../../components/Paginator.vue";
import AppSelect from "../../components/AppSelect.vue";
import {
  applyFieldRules,
  clearFieldError,
  useFormGuard,
} from "../../composables/useFormGuard";
import { usePagedList, type Paged } from "../../composables/usePagedList";
import { useToast } from "../../composables/useToast";
import { useConfirm } from "../../composables/useConfirm";
import { useAuth } from "../../composables/useAuth";
import { i18n } from "../../i18n";
import { permissionLabel as rawPermissionLabel } from "../../lib/permissions";
import type { Role, UserRecord } from "../../types";
import emptyUsers from "../../assets/empty/users.svg";

const auth = useAuth();
const toast = useToast();
const { confirmDialog } = useConfirm();
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

const search = ref("");
const roles = ref<Role[]>([]);
const permissions = ref<string[]>([]);

const {
  items: users,
  loading,
  page,
  size,
  totalItems,
  goToPage,
  reload: reloadUsers,
} = usePagedList<UserRecord>(
  (limit, offset) =>
    invoke<Paged<UserRecord>>("list_users", {
      search: search.value.trim() || null,
      limit,
      offset,
    }),
  [search],
  (e) => toast.error(e instanceof Error ? e.message : String(e)),
);


const showAdd = ref(false);
const adding = ref(false);
const addForm = ref({ username: "", fullName: "", password: "", roleId: 0 });
const addErrors = reactive<Record<string, string>>({});
const addGuard = useFormGuard(addForm);
const canAdd = computed(() => addGuard.isDirty.value && !adding.value);

function resetErrors(errors: Record<string, string>) {
  for (const key of Object.keys(errors)) delete errors[key];
}

function validateAdd(): boolean {
  const ok = applyFieldRules(addErrors, [
    ["fullName", !!addForm.value.fullName.trim(), t("users.fullName")],
    ["username", !!addForm.value.username.trim(), t("common.username")],
    ["password", !!addForm.value.password, t("common.password")],
    ["roleId", !!addForm.value.roleId, t("users.role")],
  ]);
  return ok;
}

const editTarget = ref<UserRecord | null>(null);
const saving = ref(false);
const editSelection = ref<string[]>([]);
const editForm = ref({ username: "", fullName: "", password: "", roleId: 0 });
const editErrors = reactive<Record<string, string>>({});
const editState = computed(() => ({
  username: editForm.value.username,
  fullName: editForm.value.fullName,
  password: editForm.value.password,
  roleId: editForm.value.roleId,
  perms: [...editSelection.value].sort().join(","),
}));
const editGuard = useFormGuard(editState);
const canSaveEdit = computed(() => editGuard.isDirty.value && !saving.value);

function validateEdit(): boolean {
  return applyFieldRules(editErrors, [
    ["fullName", !!editForm.value.fullName.trim(), t("users.fullName")],
    ["username", !!editForm.value.username.trim(), t("common.username")],
    ["roleId", !!editForm.value.roleId, t("users.role")],
  ]);
}

function cashierRoleId() {
  return (
    roles.value.find((r) => r.name === "Cashier")?.id ?? roles.value[0]?.id ?? 0
  );
}

async function load() {
  try {
    const [r, p] = await Promise.all([
      invoke<Role[]>("list_roles"),
      invoke<string[]>("list_permissions"),
    ]);
    roles.value = r;
    permissions.value = p;
    if (!addForm.value.roleId) addForm.value.roleId = cashierRoleId();
  } catch (e) {
    toast.error(e instanceof Error ? e.message : String(e));
  }
}

async function refresh() {
  await Promise.all([reloadUsers(), load()]);
}

function openAdd() {
  addForm.value = {
    username: "",
    fullName: "",
    password: "",
    roleId: cashierRoleId(),
  };
  resetErrors(addErrors);
  addGuard.capture();
  showAdd.value = true;
}

async function submitAdd() {
  if (!validateAdd()) {
    toast.error(t("common.fixErrors"));
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
    addGuard.markSaved();
    showAdd.value = false;
    toast.success(t("users.cashierCreated"));
    await refresh();
  } catch (e) {
    toast.error(e instanceof Error ? e.message : String(e));
  } finally {
    adding.value = false;
  }
}

function openEdit(u: UserRecord) {
  editTarget.value = u;
  editSelection.value = [...u.permissions];
  editForm.value = {
    username: u.username,
    fullName: u.fullName,
    password: "",
    roleId: u.roleId,
  };
  resetErrors(editErrors);
  editGuard.capture();
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
  if (!validateEdit()) {
    toast.error(t("common.fixErrors"));
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
    editGuard.markSaved();
    editTarget.value = null;
    toast.success(t("users.userUpdated"));
    await refresh();
  } catch (e) {
    toast.error(e instanceof Error ? e.message : String(e));
  } finally {
    saving.value = false;
  }
}

async function deactivate(u: UserRecord) {
  if (
    !(await confirmDialog({
      message: t("users.deactivateConfirm", { name: u.fullName }),
    }))
  )
    return;
  try {
    await invoke("delete_user", { userId: u.id });
    toast.success(t("users.deactivated2", { name: u.fullName }));
    await reloadUsers();
  } catch (e) {
    toast.error(e instanceof Error ? e.message : String(e));
  }
}

async function activate(u: UserRecord) {
  try {
    await invoke("set_user_active", { userId: u.id, active: true });
    toast.success(t("users.reactivated", { name: u.fullName }));
    await reloadUsers();
  } catch (e) {
    toast.error(e instanceof Error ? e.message : String(e));
  }
}

async function remove(u: UserRecord) {
  const msg = t("users.deleteConfirm", {
    name: u.fullName,
    username: u.username,
  });
  if (!(await confirmDialog({ message: msg }))) return;
  try {
    await invoke("remove_user", { userId: u.id });
    toast.success(t("users.deleted", { name: u.fullName }));
    await reloadUsers();
  } catch (e) {
    toast.error(e instanceof Error ? e.message : String(e));
  }
}

onMounted(() => {
  reloadUsers();
  load();
});
</script>
