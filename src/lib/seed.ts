import { invoke } from "@tauri-apps/api/core";
import { execute, insert, selectOne } from "./db";

const ROLE_PERMISSIONS: Record<string, string[]> = {
  Admin: [
    "sales.checkout",
    "sales.void",
    "sales.discount",
    "reports.view",
    "inventory.view",
    "expenses.manage",
    "export.excel",
    "users.manage",
    "settings.manage",
  ],
  Cashier: ["sales.checkout"],
};

export async function seedIfNeeded(): Promise<void> {
  const baseCurrency = await selectOne<{ id: number }>(
    "SELECT id FROM currencies WHERE is_base = 1 LIMIT 1"
  );
  if (!baseCurrency) {
    await execute(
      "INSERT INTO currencies (code, name, symbol, rate, is_base) VALUES ('EGP', 'Egyptian Pound', 'E£', 1, 1)"
    );
  }

  for (const [roleName, perms] of Object.entries(ROLE_PERMISSIONS)) {
    const role = await selectOne<{ id: number }>(
      "SELECT id FROM roles WHERE name = ?",
      [roleName]
    );
    const roleId = role
      ? role.id
      : await insert("INSERT INTO roles (name) VALUES (?)", [roleName]);

    for (const code of perms) {
      const perm = await selectOne<{ id: number }>(
        "SELECT id FROM permissions WHERE code = ?",
        [code]
      );
      const permId = perm
        ? perm.id
        : await insert("INSERT INTO permissions (code) VALUES (?)", [code]);

      const linked = await selectOne(
        "SELECT 1 FROM role_permissions WHERE role_id = ? AND permission_id = ?",
        [roleId, permId]
      );
      if (!linked) {
        await execute(
          "INSERT INTO role_permissions (role_id, permission_id) VALUES (?, ?)",
          [roleId, permId]
        );
      }
    }
  }

  const admin = await selectOne<{ id: number }>(
    "SELECT id FROM users WHERE username = 'admin'"
  );
  if (!admin) {
    const adminRole = await selectOne<{ id: number }>(
      "SELECT id FROM roles WHERE name = 'Admin'"
    );
    if (!adminRole) throw new Error("Admin role missing during seed");
    const hash = await invoke<string>("hash_password", { password: "admin" });
    await insert(
      "INSERT INTO users (username, password_hash, full_name, role_id) VALUES ('admin', ?, 'Administrator', ?)",
      [hash, adminRole.id]
    );
  }
}
