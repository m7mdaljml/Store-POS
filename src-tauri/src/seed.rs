use argon2::{Argon2, PasswordHash, PasswordVerifier};
use tauri::{AppHandle, Runtime};

use crate::db;

const ROLE_PERMISSIONS: &[(&str, &[&str])] = &[
    (
        "Admin",
        &[
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
    ),
    ("Cashier", &["sales.checkout"]),
];

pub async fn seed_app<R: Runtime>(app: &AppHandle<R>) -> Result<(), Box<dyn std::error::Error>> {
    let db_path = db::db_path(app)?;
    seed_db(&db_path).await
}

pub async fn seed_db(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let pool = db::connect(path).await?;

    let threshold: Option<(String,)> =
        sqlx::query_as("SELECT value FROM settings WHERE key = 'discount_threshold'")
            .fetch_optional(&pool)
            .await?;
    if threshold.is_none() {
        sqlx::query("INSERT INTO settings (key, value) VALUES ('discount_threshold', '10')")
            .execute(&pool)
            .await?;
    }

    let has_base: Option<(i64,)> =
        sqlx::query_as("SELECT id FROM currencies WHERE is_base = 1 LIMIT 1")
            .fetch_optional(&pool)
            .await?;
    if has_base.is_none() {
        sqlx::query(
            "INSERT INTO currencies (code, name, symbol, rate, is_base) VALUES ('JOD', 'Jordanian Dinar', 'JD', 1, 1)",
        )
        .execute(&pool)
        .await?;
    }

    for &(role_name, perms) in ROLE_PERMISSIONS {
        let role_id: i64 = match sqlx::query_scalar("SELECT id FROM roles WHERE name = ?")
            .bind(role_name)
            .fetch_optional(&pool)
            .await?
        {
            Some(id) => id,
            None => {
                sqlx::query("INSERT INTO roles (name) VALUES (?)")
                    .bind(role_name)
                    .execute(&pool)
                    .await?;
                sqlx::query_scalar("SELECT id FROM roles WHERE name = ?")
                    .bind(role_name)
                    .fetch_one(&pool)
                    .await?
            }
        };

        for code in perms {
            let perm_id: i64 = match sqlx::query_scalar("SELECT id FROM permissions WHERE code = ?")
                .bind(code)
                .fetch_optional(&pool)
                .await?
            {
                Some(id) => id,
                None => {
                    sqlx::query("INSERT INTO permissions (code) VALUES (?)")
                        .bind(code)
                        .execute(&pool)
                        .await?;
                    sqlx::query_scalar("SELECT id FROM permissions WHERE code = ?")
                        .bind(code)
                        .fetch_one(&pool)
                        .await?
                }
            };

            let linked: Option<(i64,)> = sqlx::query_as(
                "SELECT 1 FROM role_permissions WHERE role_id = ? AND permission_id = ?",
            )
            .bind(role_id)
            .bind(perm_id)
            .fetch_optional(&pool)
            .await?;
            if linked.is_none() {
                sqlx::query("INSERT INTO role_permissions (role_id, permission_id) VALUES (?, ?)")
                    .bind(role_id)
                    .bind(perm_id)
                    .execute(&pool)
                    .await?;
            }
        }
    }

    // The system never ships a known admin credential (e.g. admin/admin) for
    // the production database, so a copied database cannot be opened with a
    // default password. On a fresh install no user exists at all -> the
    // frontend shows a first-run setup screen where the administrator picks
    // their own username + password (see commands::auth::needs_setup and
    // setup_admin).
    //
    // For installations upgraded from an old seed that already created an
    // 'admin' user still using the insecure default password 'admin', we
    // detect it and flag Password Reset Required so the admin is forced to
    // choose a new password on their next login.
    let admin: Option<(i64, String)> =
        sqlx::query_as("SELECT id, password_hash FROM users WHERE username = 'admin'")
            .fetch_optional(&pool)
            .await?;
    if let Some((admin_id, hash)) = admin {
        if let Ok(parsed) = PasswordHash::new(&hash) {
            let is_default = Argon2::default()
                .verify_password(b"admin", &parsed)
                .is_ok();
            if is_default {
                sqlx::query("UPDATE users SET password_state = 'reset' WHERE id = ?")
                    .bind(admin_id)
                    .execute(&pool)
                    .await?;
            }
        }
    }

    pool.close().await;
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use argon2::password_hash::SaltString;
    use argon2::PasswordHasher;

    pub(crate) async fn create_schema(path: &std::path::Path) {
        let pool = db::connect(path).await.unwrap();
        for sql in [
            "CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
            "CREATE TABLE currencies (id INTEGER PRIMARY KEY AUTOINCREMENT, code TEXT UNIQUE NOT NULL, name TEXT NOT NULL, symbol TEXT NOT NULL, rate REAL NOT NULL DEFAULT 1, is_base INTEGER NOT NULL DEFAULT 0)",
            "CREATE TABLE roles (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT UNIQUE NOT NULL, description TEXT)",
            "CREATE TABLE permissions (id INTEGER PRIMARY KEY AUTOINCREMENT, code TEXT UNIQUE NOT NULL)",
            "CREATE TABLE role_permissions (role_id INTEGER NOT NULL REFERENCES roles(id) ON DELETE CASCADE, permission_id INTEGER NOT NULL REFERENCES permissions(id) ON DELETE CASCADE, PRIMARY KEY (role_id, permission_id))",
            "CREATE TABLE users (id INTEGER PRIMARY KEY AUTOINCREMENT, username TEXT UNIQUE NOT NULL, password_hash TEXT NOT NULL, full_name TEXT NOT NULL, role_id INTEGER NOT NULL REFERENCES roles(id), is_active INTEGER NOT NULL DEFAULT 1, password_state TEXT NOT NULL DEFAULT 'set')",
        ] {
            sqlx::query(sql).execute(&pool).await.unwrap();
        }
        pool.close().await;
    }

    #[tokio::test]
    async fn seed_populates_and_is_idempotent() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "store-pos-seed-test-{}-{nanos}.db",
            std::process::id()
        ));

        create_schema(&path).await;

        seed_db(&path).await.unwrap();
        seed_db(&path).await.unwrap();

        let pool = db::connect(&path).await.unwrap();
        // A fresh install must NOT ship with any users (no hard-coded admin),
        // so the frontend can present the first-run setup flow.
        let users: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM users").fetch_one(&pool).await.unwrap();
        assert_eq!(users, 0);
        let roles: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM roles").fetch_one(&pool).await.unwrap();
        assert_eq!(roles, 2);
        let perms: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM permissions")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(perms, 9);
        let links: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM role_permissions")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(links, 10);
        let threshold: String =
            sqlx::query_scalar("SELECT value FROM settings WHERE key = 'discount_threshold'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(threshold, "10");
        pool.close().await;

        std::fs::remove_file(&path).ok();
    }

    #[tokio::test]
    async fn legacy_default_admin_is_flagged_for_reset() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "store-pos-seed-reset-test-{}-{nanos}.db",
            std::process::id()
        ));
        create_schema(&path).await;
        let pool = db::connect(&path).await.unwrap();
        let _ = sqlx::query(
            "INSERT INTO roles (name) VALUES ('Admin') ON CONFLICT(name) DO NOTHING",
        )
        .execute(&pool)
        .await;
        let admin_role_id: i64 =
            sqlx::query_scalar("SELECT id FROM roles WHERE name = 'Admin'").fetch_one(&pool).await.unwrap();
        let salt = SaltString::generate(&mut argon2::password_hash::rand_core::OsRng);
        let hash = Argon2::default()
            .hash_password(b"admin", &salt)
            .unwrap()
            .to_string();
        sqlx::query(
            "INSERT INTO users (username, password_hash, full_name, role_id, password_state) VALUES ('admin', ?, 'Administrator', ?, 'set')",
        )
        .bind(&hash)
        .bind(admin_role_id)
        .execute(&pool)
        .await
        .unwrap();
        pool.close().await;

        seed_db(&path).await.unwrap();

        let pool = db::connect(&path).await.unwrap();
        let state: String = sqlx::query_scalar("SELECT password_state FROM users WHERE username = 'admin'")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(state, "reset");
        pool.close().await;

        std::fs::remove_file(&path).ok();
    }
}
