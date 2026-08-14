use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHasher, PasswordHash, PasswordVerifier};
use serde::Serialize;
use sqlx::{Row, SqlitePool};
use tauri::{AppHandle, Runtime};

use crate::db;

#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthUser {
    pub id: i64,
    pub username: String,
    pub full_name: String,
    pub role_name: String,
    pub permissions: Vec<String>,
}

fn hash_password_inner(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| e.to_string())
}

fn verify_password_inner(password: &str, hash: &str) -> Result<bool, String> {
    let parsed = PasswordHash::new(hash).map_err(|e| e.to_string())?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

async fn authenticate(pool: &SqlitePool, username: &str, password: &str) -> Result<AuthUser, String> {
    let row = sqlx::query(
        "SELECT u.id, u.password_hash, u.full_name, u.role_id, u.is_active,
                r.name AS role_name
         FROM users u
         JOIN roles r ON r.id = u.role_id
         WHERE u.username = ?",
    )
    .bind(username)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?;

    let row = match row {
        Some(row) => row,
        None => return Err("Invalid username or password".into()),
    };

    let is_active: i64 = row.try_get("is_active").map_err(|e| e.to_string())?;
    if is_active == 0 {
        return Err("This account is disabled".into());
    }

    let hash: String = row.try_get("password_hash").map_err(|e| e.to_string())?;
    if !verify_password_inner(password, &hash)? {
        return Err("Invalid username or password".into());
    }

    let id: i64 = row.try_get("id").map_err(|e| e.to_string())?;
    let full_name: String = row.try_get("full_name").map_err(|e| e.to_string())?;
    let role_id: i64 = row.try_get("role_id").map_err(|e| e.to_string())?;
    let role_name: String = row.try_get("role_name").map_err(|e| e.to_string())?;

    let perm_rows = sqlx::query(
        "SELECT p.code
         FROM permissions p
         JOIN role_permissions rp ON rp.permission_id = p.id
         WHERE rp.role_id = ?",
    )
    .bind(role_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let permissions = perm_rows
        .iter()
        .map(|r| r.try_get::<String, _>("code"))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(AuthUser {
        id,
        username: username.to_string(),
        full_name,
        role_name,
        permissions,
    })
}

#[tauri::command]
pub fn hash_password(password: String) -> Result<String, String> {
    hash_password_inner(&password)
}

#[tauri::command]
pub fn verify_password(password: String, hash: String) -> Result<bool, String> {
    verify_password_inner(&password, &hash)
}

#[tauri::command]
pub async fn login<R: Runtime>(
    app: AppHandle<R>,
    username: String,
    password: String,
) -> Result<AuthUser, String> {
    let pool = db::pool(&app).await?;
    authenticate(&pool, &username, &password).await
}

#[tauri::command]
pub fn logout() -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub async fn create_user<R: Runtime>(
    app: AppHandle<R>,
    username: String,
    password: String,
    full_name: String,
    role_id: i64,
) -> Result<i64, String> {
    let pool = db::pool(&app).await?;
    let hash = hash_password_inner(&password)?;
    let result = sqlx::query(
        "INSERT INTO users (username, password_hash, full_name, role_id) VALUES (?, ?, ?, ?)",
    )
    .bind(&username)
    .bind(&hash)
    .bind(&full_name)
    .bind(role_id)
    .execute(&pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(result.last_insert_rowid())
}

#[tauri::command]
pub async fn delete_user<R: Runtime>(app: AppHandle<R>, user_id: i64) -> Result<(), String> {
    let pool = db::pool(&app).await?;
    let result = sqlx::query("UPDATE users SET is_active = 0 WHERE id = ?")
        .bind(user_id)
        .execute(&pool)
        .await
        .map_err(|e| e.to_string())?;
    if result.rows_affected() == 0 {
        return Err("User not found".into());
    }
    Ok(())
}

#[tauri::command]
pub async fn update_user_permissions<R: Runtime>(
    app: AppHandle<R>,
    user_id: i64,
    permission_codes: Vec<String>,
) -> Result<Vec<String>, String> {
    let pool = db::pool(&app).await?;

    let role_id: Option<i64> = sqlx::query_scalar("SELECT role_id FROM users WHERE id = ?")
        .bind(user_id)
        .fetch_optional(&pool)
        .await
        .map_err(|e| e.to_string())?;
    let role_id = role_id.ok_or_else(|| "User not found".to_string())?;

    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    sqlx::query("DELETE FROM role_permissions WHERE role_id = ?")
        .bind(role_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    for code in &permission_codes {
        let perm_id: Option<i64> =
            sqlx::query_scalar("SELECT id FROM permissions WHERE code = ?")
                .bind(code)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
        let perm_id = match perm_id {
            Some(id) => id,
            None => {
                sqlx::query("INSERT INTO permissions (code) VALUES (?)")
                    .bind(code)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| e.to_string())?;
                sqlx::query_scalar("SELECT id FROM permissions WHERE code = ?")
                    .bind(code)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(|e| e.to_string())?
            }
        };
        sqlx::query("INSERT INTO role_permissions (role_id, permission_id) VALUES (?, ?)")
            .bind(role_id)
            .bind(perm_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
    }

    tx.commit().await.map_err(|e| e.to_string())?;

    Ok(permission_codes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seed;

    #[tokio::test]
    async fn authenticate_works_with_seeded_db() {
        let path = std::env::temp_dir().join(format!(
            "store-pos-auth-test-{}.db",
            std::process::id()
        ));
        seed::tests::create_schema(&path).await;
        seed::seed_db(&path).await.unwrap();
        let pool = db::connect(&path).await.unwrap();

        let user = authenticate(&pool, "admin", "admin").await.unwrap();
        assert_eq!(user.username, "admin");
        assert_eq!(user.full_name, "Administrator");
        assert_eq!(user.role_name, "Admin");
        assert!(user.permissions.contains(&"users.manage".to_string()));

        assert!(authenticate(&pool, "admin", "wrong").await.is_err());
        assert!(authenticate(&pool, "nobody", "admin").await.is_err());

        std::fs::remove_file(&path).ok();
    }
}
