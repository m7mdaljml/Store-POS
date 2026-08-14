use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHasher};
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

    let has_base: Option<(i64,)> =
        sqlx::query_as("SELECT id FROM currencies WHERE is_base = 1 LIMIT 1")
            .fetch_optional(&pool)
            .await?;
    if has_base.is_none() {
        sqlx::query(
            "INSERT INTO currencies (code, name, symbol, rate, is_base) VALUES ('EGP', 'Egyptian Pound', 'E\u{00a3}', 1, 1)",
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

    let admin: Option<(i64,)> = sqlx::query_as("SELECT id FROM users WHERE username = 'admin'")
        .fetch_optional(&pool)
        .await?;
    if admin.is_none() {
        let admin_role_id: i64 = sqlx::query_scalar("SELECT id FROM roles WHERE name = 'Admin'")
            .fetch_one(&pool)
            .await?;
        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password(b"admin", &salt)
            .map_err(|e| e.to_string())?
            .to_string();
        sqlx::query(
            "INSERT INTO users (username, password_hash, full_name, role_id) VALUES ('admin', ?, 'Administrator', ?)",
        )
        .bind(hash)
        .bind(admin_role_id)
        .execute(&pool)
        .await?;
    }

    pool.close().await;
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use argon2::{PasswordHash, PasswordVerifier};

    pub(crate) async fn create_schema(path: &std::path::Path) {
        let pool = db::connect(path).await.unwrap();
        for sql in [
            "CREATE TABLE currencies (id INTEGER PRIMARY KEY AUTOINCREMENT, code TEXT UNIQUE NOT NULL, name TEXT NOT NULL, symbol TEXT NOT NULL, rate REAL NOT NULL DEFAULT 1, is_base INTEGER NOT NULL DEFAULT 0)",
            "CREATE TABLE roles (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT UNIQUE NOT NULL, description TEXT)",
            "CREATE TABLE permissions (id INTEGER PRIMARY KEY AUTOINCREMENT, code TEXT UNIQUE NOT NULL)",
            "CREATE TABLE role_permissions (role_id INTEGER NOT NULL REFERENCES roles(id) ON DELETE CASCADE, permission_id INTEGER NOT NULL REFERENCES permissions(id) ON DELETE CASCADE, PRIMARY KEY (role_id, permission_id))",
            "CREATE TABLE users (id INTEGER PRIMARY KEY AUTOINCREMENT, username TEXT UNIQUE NOT NULL, password_hash TEXT NOT NULL, full_name TEXT NOT NULL, role_id INTEGER NOT NULL REFERENCES roles(id), is_active INTEGER NOT NULL DEFAULT 1)",
        ] {
            sqlx::query(sql).execute(&pool).await.unwrap();
        }
        pool.close().await;
    }

    #[tokio::test]
    async fn seed_populates_and_is_idempotent() {
        let path = std::env::temp_dir().join(format!(
            "store-pos-seed-test-{}.db",
            std::process::id()
        ));

        create_schema(&path).await;

        seed_db(&path).await.unwrap();
        seed_db(&path).await.unwrap();

        let pool = db::connect(&path).await.unwrap();
        let users: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM users").fetch_one(&pool).await.unwrap();
        assert_eq!(users, 1);
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
        let hash: String = sqlx::query_scalar("SELECT password_hash FROM users WHERE username = 'admin'")
            .fetch_one(&pool)
            .await
            .unwrap();
        let parsed = PasswordHash::new(&hash).unwrap();
        assert!(Argon2::default().verify_password(b"admin", &parsed).is_ok());
        pool.close().await;

        std::fs::remove_file(&path).ok();
    }
}
