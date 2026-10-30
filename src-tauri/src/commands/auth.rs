use argon2::password_hash::rand_core::{OsRng, RngCore};
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHasher, PasswordHash, PasswordVerifier};
use serde::Serialize;
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;
use tauri::{AppHandle, Runtime};

use super::guard;
use super::Page;
use crate::db;

#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthUser {
    pub id: i64,
    pub username: String,
    pub full_name: String,
    pub role_name: String,
    pub permissions: Vec<String>,
    /// True when the user signed in with a temporary/initial password and must
    /// choose a permanent one before using the POS (Pending Activation or
    /// Password Reset Required).
    pub must_change_password: bool,
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
    pub password_state: String,
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

/// Password hashing helper exposed to tests and other Rust modules. This is
/// intentionally NOT a Tauri command: the webview must never hash or verify
/// passwords itself.
pub fn hash_password(password: String) -> Result<String, String> {
    hash_password_inner(&password)
}

/// Password verification helper exposed to tests and other Rust modules.
pub fn verify_password(password: String, hash: String) -> Result<bool, String> {
    verify_password_inner(&password, &hash)
}

/// A temporary/reset password is only ever shown once to an administrator so
/// they can hand it to the user; it is replaced the moment the user chooses
/// their own permanent password. We deliberately use a cryptographically
/// random token built from a human-friendly character set.
fn generate_temporary_password() -> String {
    const CHARSET: &[u8] =
        b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghjkmnpqrstuvwxyz23456789";
    let mut rng = OsRng;
    let mut pw = String::with_capacity(10);
    for _ in 0..10 {
        let idx = (rng.next_u32() as usize) % CHARSET.len();
        pw.push(CHARSET[idx] as char);
    }
    pw
}

/// Resolves the permissions that actually apply to a user.
///
/// Per-user overrides (the `user_permissions` table) take precedence when any
/// exist; otherwise the user inherits their role's permissions. An empty
/// override set means "inherit from role" rather than "no permissions", which
/// keeps the default behaviour intact.
async fn effective_permissions(
    pool: &SqlitePool,
    user_id: i64,
    role_id: i64,
) -> Result<Vec<String>, String> {
    let overrides = sqlx::query(
        "SELECT p.code
         FROM user_permissions up
         JOIN permissions p ON p.id = up.permission_id
         WHERE up.user_id = ?
         ORDER BY p.code",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    if !overrides.is_empty() {
        return overrides
            .iter()
            .map(|r| r.try_get::<String, _>("code").map_err(|e| e.to_string()))
            .collect();
    }

    let role_rows = sqlx::query(
        "SELECT p.code
         FROM permissions p
         JOIN role_permissions rp ON rp.permission_id = p.id
         WHERE rp.role_id = ?
         ORDER BY p.code",
    )
    .bind(role_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    role_rows
        .iter()
        .map(|r| r.try_get::<String, _>("code").map_err(|e| e.to_string()))
        .collect()
}

async fn authenticate(pool: &SqlitePool, username: &str, password: &str) -> Result<AuthUser, String> {
    let row = sqlx::query(
        "SELECT u.id, u.password_hash, u.full_name, u.role_id, u.is_active,
                u.password_state, r.name AS role_name
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

    let must_change_password =
        matches!(row.try_get::<String, _>("password_state").map_err(|e| e.to_string())?.as_str(), "pending" | "reset");

    let id: i64 = row.try_get("id").map_err(|e| e.to_string())?;
    let full_name: String = row.try_get("full_name").map_err(|e| e.to_string())?;
    let role_id: i64 = row.try_get("role_id").map_err(|e| e.to_string())?;
    let role_name: String = row.try_get("role_name").map_err(|e| e.to_string())?;

    let permissions = effective_permissions(pool, id, role_id).await?;

    Ok(AuthUser {
        id,
        username: username.to_string(),
        full_name,
        role_name,
        permissions,
        must_change_password,
    })
}

#[tauri::command]
pub async fn login<R: Runtime>(
    app: AppHandle<R>,
    username: String,
    password: String,
) -> Result<AuthUser, String> {
    let pool = db::pool(&app).await?;
    let user = authenticate(&pool, &username, &password).await?;
    // Record the authoritative session on the Rust side so every privileged
    // command can verify identity and permissions without trusting the webview.
    let role_id: i64 = sqlx::query_scalar("SELECT role_id FROM users WHERE id = ?")
        .bind(user.id)
        .fetch_one(&pool)
        .await
        .map_err(|e| e.to_string())?;
    guard::set(
        &app,
        guard::Session {
            user_id: user.id,
            role_id,
            role_name: user.role_name.clone(),
            permissions: user.permissions.clone(),
            must_change_password: user.must_change_password,
        },
    );
    Ok(user)
}

#[tauri::command]
pub fn logout<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    guard::clear(&app);
    Ok(())
}

#[tauri::command]
pub async fn verify_session<R: Runtime>(
    app: AppHandle<R>,
    user_id: i64,
) -> Result<bool, String> {
    let Some(session) = guard::current(&app) else {
        return Ok(false);
    };
    if session.user_id != user_id {
        return Ok(false);
    }
    let pool = db::pool(&app).await?;
    let active: Option<(i64,)> =
        sqlx::query_as("SELECT id FROM users WHERE id = ? AND is_active = 1")
            .bind(user_id)
            .fetch_optional(&pool)
            .await
            .map_err(|e| e.to_string())?;
    if active.is_none() {
        guard::clear(&app);
        return Ok(false);
    }
    Ok(true)
}

#[tauri::command]
pub async fn create_user<R: Runtime>(
    app: AppHandle<R>,
    username: String,
    full_name: String,
    role_id: i64,
    created_by: Option<i64>,
) -> Result<String, String> {
    let actor = guard::require(&app, Some("users.manage"))?;
    let _ = created_by;
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
        sqlx::query_as("SELECT id FROM users WHERE username = ?")
            .bind(&username)
            .fetch_optional(&pool)
            .await
            .map_err(|e| e.to_string())?;
    if taken.is_some() {
        return Err("Username already taken".into());
    }

    // The admin never sets or knows the user's permanent password. We create
    // the account in "Pending Activation" with a generated temporary password
    // that is returned exactly once (to be handed to the user). The user must
    // choose their own password on first login.
    let temporary = generate_temporary_password();
    let hash = hash_password_inner(&temporary)?;
    let result = sqlx::query(
        "INSERT INTO users (username, password_hash, full_name, role_id, password_state)
         VALUES (?, ?, ?, ?, 'pending')",
    )
    .bind(&username)
    .bind(&hash)
    .bind(&full_name)
    .bind(role_id)
    .execute(&pool)
    .await
    .map_err(|e| e.to_string())?;
    let id = result.last_insert_rowid();
    sqlx::query(
        "INSERT INTO audit_log (user_id, action, entity_type, entity_id, details)
         VALUES (?, 'user.create', 'user', ?, ?)",
    )
    .bind(actor.user_id)
    .bind(id)
    .bind(format!("Created user \"{username}\" ({full_name})"))
    .execute(&pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(temporary)
}

#[tauri::command]
pub async fn delete_user<R: Runtime>(app: AppHandle<R>, user_id: i64, deleted_by: Option<i64>) -> Result<(), String> {
    let actor = guard::require(&app, Some("users.manage"))?;
    let _ = deleted_by;
    let pool = db::pool(&app).await?;
    let result = sqlx::query("UPDATE users SET is_active = 0 WHERE id = ?")
        .bind(user_id)
        .execute(&pool)
        .await
        .map_err(|e| e.to_string())?;
    if result.rows_affected() == 0 {
        return Err("User not found".into());
    }
    sqlx::query(
        "INSERT INTO audit_log (user_id, action, entity_type, entity_id, details)
         VALUES (?, 'user.deactivate', 'user', ?, ?)",
    )
    .bind(actor.user_id)
    .bind(user_id)
    .bind(format!("Deactivated user {user_id}"))
    .execute(&pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn set_user_active<R: Runtime>(
    app: AppHandle<R>,
    user_id: i64,
    active: bool,
    toggled_by: Option<i64>,
) -> Result<(), String> {
    let actor = guard::require(&app, Some("users.manage"))?;
    let _ = toggled_by;
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
    let action = if active { "user.activate" } else { "user.deactivate" };
    let detail = if active {
        format!("Reactivated user {user_id}")
    } else {
        format!("Deactivated user {user_id}")
    };
    sqlx::query(
        "INSERT INTO audit_log (user_id, action, entity_type, entity_id, details)
         VALUES (?, ?, 'user', ?, ?)",
    )
    .bind(actor.user_id)
    .bind(action)
    .bind(user_id)
    .bind(detail)
    .execute(&pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn remove_user<R: Runtime>(app: AppHandle<R>, user_id: i64) -> Result<(), String> {
    let actor = guard::require(&app, Some("users.manage"))?;
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

    sqlx::query(
        "INSERT INTO audit_log (user_id, action, entity_type, entity_id, details)
         VALUES (?, 'user.delete', 'user', ?, ?)",
    )
    .bind(actor.user_id)
    .bind(user_id)
    .bind(format!("Permanently deleted user {user_id}"))
    .execute(&pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn update_user<R: Runtime>(
    app: AppHandle<R>,
    user_id: i64,
    username: String,
    full_name: String,
    role_id: i64,
    updated_by: Option<i64>,
) -> Result<(), String> {
    let actor = guard::require(&app, Some("users.manage"))?;
    let _ = updated_by;
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

    // The admin can update profile fields and role but never the password.
    let result = sqlx::query(
        "UPDATE users SET username = ?, full_name = ?, role_id = ? WHERE id = ?",
    )
    .bind(&username)
    .bind(&full_name)
    .bind(role_id)
    .bind(user_id)
    .execute(&pool)
    .await
    .map_err(|e| e.to_string())?;

    if result.rows_affected() == 0 {
        return Err("User not found".into());
    }
    sqlx::query(
        "INSERT INTO audit_log (user_id, action, entity_type, entity_id, details)
         VALUES (?, 'user.update', 'user', ?, ?)",
    )
    .bind(actor.user_id)
    .bind(user_id)
    .bind(format!("Updated user \"{username}\""))
    .execute(&pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Returns whether the database is on first run (no users exist yet), which
/// triggers the first-install administrator setup screen.
#[tauri::command]
pub async fn needs_setup<R: Runtime>(app: AppHandle<R>) -> Result<bool, String> {
    let pool = db::pool(&app).await?;
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM users").fetch_one(&pool).await.map_err(|e| e.to_string())?;
    Ok(count == 0)
}

/// First-install: creates the initial administrator with a username and
/// password of the owner's choosing (never a hard-coded credential). Passwords
/// are stored only as an argon2 hash.
#[tauri::command]
pub async fn setup_admin<R: Runtime>(
    app: AppHandle<R>,
    username: String,
    password: String,
    full_name: String,
) -> Result<(), String> {
    let pool = db::pool(&app).await?;

    let username = username.trim().to_string();
    let full_name = full_name.trim().to_string();
    if username.is_empty() {
        return Err("Username is required".into());
    }
    if password.len() < 4 {
        return Err("Password must be at least 4 characters".into());
    }
    if full_name.is_empty() {
        return Err("Full name is required".into());
    }

    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM users").fetch_one(&pool).await.map_err(|e| e.to_string())?;
    if count > 0 {
        return Err("Setup has already been completed".into());
    }
    let taken: Option<(i64,)> =
        sqlx::query_as("SELECT id FROM users WHERE username = ?")
            .bind(&username)
            .fetch_optional(&pool)
            .await
            .map_err(|e| e.to_string())?;
    if taken.is_some() {
        return Err("Username already taken".into());
    }

    let admin_role_id: i64 = sqlx::query_scalar("SELECT id FROM roles WHERE name = 'Admin'")
        .fetch_one(&pool)
        .await
        .map_err(|e| format!("Admin role is missing: {e}"))?;

    let hash = hash_password_inner(&password)?;
    sqlx::query(
        "INSERT INTO users (username, password_hash, full_name, role_id, password_state)
         VALUES (?, ?, ?, ?, 'set')",
    )
    .bind(&username)
    .bind(&hash)
    .bind(&full_name)
    .bind(admin_role_id)
    .execute(&pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Lets a user set their own permanent password. Called from the forced
/// "choose a new password" screen after first login (Pending Activation) or
/// after an admin reset. The temporary password is immediately invalidated
/// because it is overwritten by the new hash.
#[tauri::command]
pub async fn set_own_password<R: Runtime>(
    app: AppHandle<R>,
    user_id: i64,
    new_password: String,
) -> Result<(), String> {
    let session = guard::require_session(&app)?;
    // The password can only be changed for the signed-in account; ignore any
    // other id the webview might send.
    let user_id = if user_id == session.user_id {
        user_id
    } else {
        session.user_id
    };
    let pool = db::pool(&app).await?;
    if new_password.len() < 4 {
        return Err("Password must be at least 4 characters".into());
    }
    let hash = hash_password_inner(&new_password)?;
    let result = sqlx::query(
        "UPDATE users SET password_hash = ?, password_state = 'set' WHERE id = ?",
    )
    .bind(&hash)
    .bind(user_id)
    .execute(&pool)
    .await
    .map_err(|e| e.to_string())?;
    if result.rows_affected() == 0 {
        return Err("User not found".into());
    }
    sqlx::query(
        "INSERT INTO audit_log (user_id, action, entity_type, entity_id, details)
         VALUES (?, 'user.password_set', 'user', ?, ?)",
    )
    .bind(user_id)
    .bind(user_id)
    .bind("User set their own password")
    .execute(&pool)
    .await
    .map_err(|e| e.to_string())?;
    guard::mark_password_changed(&app);
    Ok(())
}

/// Admin-only: resets a user's password. A new temporary password is generated
/// and returned exactly once (to be handed to the user); the user must choose a
/// new permanent password at their next login. The existing password is never
/// disclosed.
#[tauri::command]
pub async fn reset_user_password<R: Runtime>(
    app: AppHandle<R>,
    user_id: i64,
    reset_by: Option<i64>,
) -> Result<String, String> {
    let actor = guard::require(&app, Some("users.manage"))?;
    let _ = reset_by;
    let pool = db::pool(&app).await?;
    let temporary = generate_temporary_password();
    let hash = hash_password_inner(&temporary)?;
    let result = sqlx::query(
        "UPDATE users SET password_hash = ?, password_state = 'reset' WHERE id = ?",
    )
    .bind(&hash)
    .bind(user_id)
    .execute(&pool)
    .await
    .map_err(|e| e.to_string())?;
    if result.rows_affected() == 0 {
        return Err("User not found".into());
    }
    sqlx::query(
        "INSERT INTO audit_log (user_id, action, entity_type, entity_id, details)
         VALUES (?, 'user.password_reset', 'user', ?, ?)",
    )
    .bind(actor.user_id)
    .bind(user_id)
    .bind("Admin reset user password (temporary credential issued)")
    .execute(&pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(temporary)
}

#[tauri::command]
pub async fn update_user_permissions<R: Runtime>(
    app: AppHandle<R>,
    user_id: i64,
    permission_codes: Vec<String>,
    updated_by: Option<i64>,
) -> Result<Vec<String>, String> {
    let actor = guard::require(&app, Some("users.manage"))?;
    let _ = updated_by;
    let pool = db::pool(&app).await?;

    let role_id: Option<i64> = sqlx::query_scalar("SELECT role_id FROM users WHERE id = ?")
        .bind(user_id)
        .fetch_optional(&pool)
        .await
        .map_err(|e| e.to_string())?;
    let role_id = role_id.ok_or_else(|| "User not found".to_string())?;

    // Distinct, sorted desired codes.
    let mut desired: Vec<String> = permission_codes
        .iter()
        .map(|c| c.trim().to_string())
        .filter(|c| !c.is_empty())
        .collect();
    desired.sort();
    desired.dedup();

    // If the selection matches the role's own set exactly, remove any per-user
    // overrides so the user inherits from the role again.
    let role_codes: Vec<String> = sqlx::query_scalar(
        "SELECT p.code
         FROM permissions p
         JOIN role_permissions rp ON rp.permission_id = p.id
         WHERE rp.role_id = ?
         ORDER BY p.code",
    )
    .bind(role_id)
    .fetch_all(&pool)
    .await
    .map_err(|e| e.to_string())?;
    let mut role_codes_sorted = role_codes.clone();
    role_codes_sorted.sort();
    role_codes_sorted.dedup();
    let inherits = desired == role_codes_sorted;

    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    sqlx::query("DELETE FROM user_permissions WHERE user_id = ?")
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    if !inherits {
        for code in &desired {
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
            sqlx::query("INSERT INTO user_permissions (user_id, permission_id) VALUES (?, ?)")
                .bind(user_id)
                .bind(perm_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
        }
    }

    tx.commit().await.map_err(|e| e.to_string())?;

    sqlx::query(
        "INSERT INTO audit_log (user_id, action, entity_type, entity_id, details)
         VALUES (?, 'user.permissions', 'user', ?, ?)",
    )
    .bind(actor.user_id)
    .bind(user_id)
    .bind(format!(
        "Updated permissions for user {user_id}: {} permission(s){}",
        desired.len(),
        if inherits { " (inherited from role)" } else { "" }
    ))
    .execute(&pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(if inherits { role_codes } else { desired })
}

#[tauri::command]
pub async fn list_roles<R: Runtime>(app: AppHandle<R>) -> Result<Vec<RoleRecord>, String> {
    guard::require(&app, None)?;
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
    guard::require(&app, None)?;
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
pub async fn list_users<R: Runtime>(
    app: AppHandle<R>,
    search: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Page<UserRecord>, String> {
    guard::require(&app, Some("users.manage"))?;
    let pool = db::pool(&app).await?;

    let pattern = search
        .as_deref()
        .map(|s| format!("%{}%", s.trim()))
        .filter(|p| p != "%%");
    let search_cond = if pattern.is_some() {
        " AND (u.username LIKE ? OR u.full_name LIKE ?)"
    } else {
        ""
    };
    let sql = format!(
        "SELECT u.id, u.username, u.full_name, u.role_id, u.is_active,
                u.password_state, r.name AS role_name
         FROM users u
         JOIN roles r ON r.id = u.role_id
         WHERE 1=1{search_cond}
         ORDER BY u.is_active DESC, u.username ASC
         LIMIT ? OFFSET ?"
    );

    let mut query = sqlx::query(&sql);
    if let Some(p) = &pattern {
        query = query.bind(p).bind(p);
    }
    let user_rows = query
        .bind(limit.map(|l| l.max(1)).unwrap_or(-1))
        .bind(offset.unwrap_or(0).max(0))
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

    // Per-user overrides win over the role defaults when present.
    let override_rows = sqlx::query(
        "SELECT up.user_id, p.code
         FROM user_permissions up
         JOIN permissions p ON p.id = up.permission_id",
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| e.to_string())?;
    let mut perms_by_user: HashMap<i64, Vec<String>> = HashMap::new();
    for row in override_rows {
        let uid: i64 = row.try_get("user_id").map_err(|e| e.to_string())?;
        let code: String = row.try_get("code").map_err(|e| e.to_string())?;
        perms_by_user.entry(uid).or_default().push(code);
    }

    let items: Vec<UserRecord> = user_rows
        .into_iter()
        .map(|row| -> Result<UserRecord, String> {
            let id: i64 = row.try_get("id").map_err(|e| e.to_string())?;
            let role_id: i64 = row.try_get("role_id").map_err(|e| e.to_string())?;
            let permissions = perms_by_user
                .get(&id)
                .filter(|v| !v.is_empty())
                .cloned()
                .unwrap_or_else(|| perms_by_role.get(&role_id).cloned().unwrap_or_default());
            let mut permissions = permissions;
            permissions.sort();
            permissions.dedup();
            Ok(UserRecord {
                id,
                username: row.try_get("username").map_err(|e| e.to_string())?,
                full_name: row.try_get("full_name").map_err(|e| e.to_string())?,
                role_id,
                role_name: row.try_get("role_name").map_err(|e| e.to_string())?,
                is_active: {
                    let v: i64 = row.try_get("is_active").map_err(|e| e.to_string())?;
                    v != 0
                },
                password_state: row.try_get("password_state").map_err(|e| e.to_string())?,
                permissions,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    let count_sql = format!(
        "SELECT COUNT(*) FROM users u JOIN roles r ON r.id = u.role_id WHERE 1=1{search_cond}"
    );
    let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
    if let Some(p) = &pattern {
        count_query = count_query.bind(p).bind(p);
    }
    let total = count_query
        .fetch_one(&pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(Page { items, total })
}

/// Minimal active-user list for filter dropdowns (e.g. the withdrawals screen).
/// Deliberately narrow - exposes only id and name, guarded by `reports.view`
/// rather than `users.manage` so reporting screens can attribute records
/// without granting user administration.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserOption {
    pub id: i64,
    pub full_name: String,
}

#[tauri::command]
pub async fn list_user_options<R: Runtime>(app: AppHandle<R>) -> Result<Vec<UserOption>, String> {
    guard::require(&app, Some("reports.view"))?;
    let pool = db::pool(&app).await?;
    let rows = sqlx::query(
        "SELECT id, full_name FROM users WHERE is_active = 1 ORDER BY full_name COLLATE NOCASE",
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| e.to_string())?;
    rows.into_iter()
        .map(|row| {
            Ok(UserOption {
                id: row.try_get("id").map_err(|e| e.to_string())?,
                full_name: row.try_get("full_name").map_err(|e| e.to_string())?,
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

        let admin_role_id: i64 =
            sqlx::query_scalar("SELECT id FROM roles WHERE name = 'Admin'")
                .fetch_one(&pool)
                .await
                .unwrap();
        let hash = hash_password_inner("S0mePass!").unwrap();
        sqlx::query(
            "INSERT INTO users (username, password_hash, full_name, role_id, password_state)
             VALUES ('admin', ?, 'Administrator', ?, 'set')",
        )
        .bind(&hash)
        .bind(admin_role_id)
        .execute(&pool)
        .await
        .unwrap();

        let user = authenticate(&pool, "admin", "S0mePass!").await.unwrap();
        assert_eq!(user.username, "admin");
        assert_eq!(user.full_name, "Administrator");
        assert_eq!(user.role_name, "Admin");
        assert!(!user.must_change_password);
        assert!(user.permissions.contains(&"users.manage".to_string()));

        assert!(authenticate(&pool, "admin", "wrong").await.is_err());
        assert!(authenticate(&pool, "nobody", "S0mePass!").await.is_err());

        std::fs::remove_file(&path).ok();
    }

    #[tokio::test]
    async fn pending_or_reset_user_must_change_password() {
        let path = std::env::temp_dir().join(format!(
            "store-pos-auth-pending-test-{}.db",
            std::process::id()
        ));
        seed::tests::create_schema(&path).await;
        seed::seed_db(&path).await.unwrap();
        let pool = db::connect(&path).await.unwrap();

        let cashier_role_id: i64 =
            sqlx::query_scalar("SELECT id FROM roles WHERE name = 'Cashier'")
                .fetch_one(&pool)
                .await
                .unwrap();
        let hash = hash_password_inner("TempPass1").unwrap();
        sqlx::query(
            "INSERT INTO users (username, password_hash, full_name, role_id, password_state)
             VALUES ('cash', ?, 'Cashier', ?, 'pending')",
        )
        .bind(&hash)
        .bind(cashier_role_id)
        .execute(&pool)
        .await
        .unwrap();

        let user = authenticate(&pool, "cash", "TempPass1").await.unwrap();
        assert!(user.must_change_password);

        std::fs::remove_file(&path).ok();
    }
}
