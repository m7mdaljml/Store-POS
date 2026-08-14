use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHasher, PasswordHash, PasswordVerifier};
use serde::Serialize;
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleRecord {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserRecord {
    pub id: i64,
    pub username: String,
    pub full_name: String,
    pub role_id: i64,
    pub role_name: String,
    pub is_active: bool,
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
pub async fn verify_session<R: Runtime>(
    app: AppHandle<R>,
    user_id: i64,
) -> Result<bool, String> {
    let pool = db::pool(&app).await?;
    let active: Option<(i64,)> =
        sqlx::query_as("SELECT id FROM users WHERE id = ? AND is_active = 1")
            .bind(user_id)
            .fetch_optional(&pool)
            .await
            .map_err(|e| e.to_string())?;
    Ok(active.is_some())
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
pub async fn set_user_active<R: Runtime>(
    app: AppHandle<R>,
    user_id: i64,
    active: bool,
) -> Result<(), String> {
    let pool = db::pool(&app).await?;
    let result = sqlx::query("UPDATE users SET is_active = ? WHERE id = ?")
        .bind(i64::from(active))
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
pub async fn remove_user<R: Runtime>(app: AppHandle<R>, user_id: i64) -> Result<(), String> {
    let pool = db::pool(&app).await?;

    let references: (i64,) = sqlx::query_as(
        "SELECT
            (SELECT COUNT(*) FROM sales WHERE user_id = ?) +
            (SELECT COUNT(*) FROM sale_sessions WHERE user_id = ?)",
    )
    .bind(user_id)
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .map_err(|e| e.to_string())?;

    if references.0 > 0 {
        return Err(
            "This user has sales history and cannot be deleted. Deactivate them instead.".into(),
        );
    }

    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    let null_queries = [
        "UPDATE sales SET voided_by = NULL WHERE voided_by = ?",
        "UPDATE stock_movements SET user_id = NULL WHERE user_id = ?",
        "UPDATE supplier_invoices SET user_id = NULL WHERE user_id = ?",
        "UPDATE supplier_payments SET user_id = NULL WHERE user_id = ?",
        "UPDATE expense_out SET user_id = NULL WHERE user_id = ?",
        "UPDATE customer_ledger SET user_id = NULL WHERE user_id = ?",
        "UPDATE audit_log SET user_id = NULL WHERE user_id = ?",
    ];
    for query in null_queries {
        sqlx::query(query)
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
    }

    let result = sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    tx.commit().await.map_err(|e| e.to_string())?;

    if result.rows_affected() == 0 {
        return Err("User not found".into());
    }
    Ok(())
}

#[tauri::command]
pub async fn update_user<R: Runtime>(
    app: AppHandle<R>,
    user_id: i64,
    username: String,
    full_name: String,
    password: Option<String>,
    role_id: i64,
) -> Result<(), String> {
    let pool = db::pool(&app).await?;

    let username = username.trim().to_string();
    let full_name = full_name.trim().to_string();
    if username.is_empty() {
        return Err("Username is required".into());
    }
    if full_name.is_empty() {
        return Err("Full name is required".into());
    }

    let taken: Option<(i64,)> =
        sqlx::query_as("SELECT id FROM users WHERE username = ? AND id != ?")
            .bind(&username)
            .bind(user_id)
            .fetch_optional(&pool)
            .await
            .map_err(|e| e.to_string())?;
    if taken.is_some() {
        return Err("Username already taken".into());
    }

    let result = match password.filter(|p| !p.is_empty()) {
        Some(pw) => {
            let hash = hash_password_inner(&pw)?;
            sqlx::query(
                "UPDATE users SET username = ?, full_name = ?, password_hash = ?, role_id = ?
                 WHERE id = ?",
            )
            .bind(&username)
            .bind(&full_name)
            .bind(&hash)
            .bind(role_id)
            .bind(user_id)
            .execute(&pool)
            .await
            .map_err(|e| e.to_string())?
        }
        None => {
            sqlx::query(
                "UPDATE users SET username = ?, full_name = ?, role_id = ? WHERE id = ?",
            )
            .bind(&username)
            .bind(&full_name)
            .bind(role_id)
            .bind(user_id)
            .execute(&pool)
            .await
            .map_err(|e| e.to_string())?
        }
    };

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

#[tauri::command]
pub async fn list_roles<R: Runtime>(app: AppHandle<R>) -> Result<Vec<RoleRecord>, String> {
    let pool = db::pool(&app).await?;
    let rows = sqlx::query("SELECT id, name FROM roles ORDER BY name")
        .fetch_all(&pool)
        .await
        .map_err(|e| e.to_string())?;
    rows.into_iter()
        .map(|row| {
            Ok(RoleRecord {
                id: row.try_get("id").map_err(|e| e.to_string())?,
                name: row.try_get("name").map_err(|e| e.to_string())?,
            })
        })
        .collect()
}

#[tauri::command]
pub async fn list_permissions<R: Runtime>(app: AppHandle<R>) -> Result<Vec<String>, String> {
    let pool = db::pool(&app).await?;
    let rows = sqlx::query("SELECT code FROM permissions ORDER BY code")
        .fetch_all(&pool)
        .await
        .map_err(|e| e.to_string())?;
    rows.into_iter()
        .map(|row| row.try_get::<String, _>("code").map_err(|e| e.to_string()))
        .collect()
}

#[tauri::command]
pub async fn list_users<R: Runtime>(app: AppHandle<R>) -> Result<Vec<UserRecord>, String> {
    let pool = db::pool(&app).await?;

    let user_rows = sqlx::query(
        "SELECT u.id, u.username, u.full_name, u.role_id, u.is_active,
                r.name AS role_name
         FROM users u
         JOIN roles r ON r.id = u.role_id
         ORDER BY u.is_active DESC, u.username ASC",
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| e.to_string())?;

    let perm_rows = sqlx::query(
        "SELECT rp.role_id, p.code
         FROM role_permissions rp
         JOIN permissions p ON p.id = rp.permission_id",
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| e.to_string())?;

    let mut perms_by_role: HashMap<i64, Vec<String>> = HashMap::new();
    for row in perm_rows {
        let role_id: i64 = row.try_get("role_id").map_err(|e| e.to_string())?;
        let code: String = row.try_get("code").map_err(|e| e.to_string())?;
        perms_by_role.entry(role_id).or_default().push(code);
    }

    user_rows
        .into_iter()
        .map(|row| {
            let role_id: i64 = row.try_get("role_id").map_err(|e| e.to_string())?;
            Ok(UserRecord {
                id: row.try_get("id").map_err(|e| e.to_string())?,
                username: row.try_get("username").map_err(|e| e.to_string())?,
                full_name: row.try_get("full_name").map_err(|e| e.to_string())?,
                role_id,
                role_name: row.try_get("role_name").map_err(|e| e.to_string())?,
                is_active: {
                    let v: i64 = row.try_get("is_active").map_err(|e| e.to_string())?;
                    v != 0
                },
                permissions: perms_by_role.get(&role_id).cloned().unwrap_or_default(),
            })
        })
        .collect()
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
