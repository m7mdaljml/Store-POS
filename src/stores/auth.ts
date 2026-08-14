import { computed, ref } from "vue";
import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { select, selectOne } from "../lib/db";
import type { AuthUser } from "../types";

export const useAuthStore = defineStore("auth", () => {
  const user = ref<AuthUser | null>(null);

  const isAuthenticated = computed(() => user.value !== null);

  async function login(username: string, password: string): Promise<void> {
    const row = await selectOne<{
      id: number;
      username: string;
      full_name: string;
      role_id: number;
      role_name: string;
      password_hash: string;
      is_active: number;
    }>(
      `SELECT u.id, u.username, u.full_name, u.role_id, u.is_active,
              u.password_hash, r.name AS role_name
       FROM users u
       JOIN roles r ON r.id = u.role_id
       WHERE u.username = ?`,
      [username]
    );

    if (!row) throw new Error("Invalid username or password");
    if (!row.is_active) throw new Error("This account is disabled");

    const valid = await invoke<boolean>("verify_password", {
      password,
      hash: row.password_hash,
    });
    if (!valid) throw new Error("Invalid username or password");

    const perms = await select<{ code: string }>(
      `SELECT p.code
       FROM permissions p
       JOIN role_permissions rp ON rp.permission_id = p.id
       WHERE rp.role_id = ?`,
      [row.role_id]
    );

    user.value = {
      id: row.id,
      username: row.username,
      fullName: row.full_name,
      roleName: row.role_name,
      permissions: perms.map((p) => p.code),
    };
  }

  function logout() {
    user.value = null;
  }

  function can(code: string): boolean {
    return user.value?.permissions.includes(code) ?? false;
  }

  return { user, isAuthenticated, login, logout, can };
});
