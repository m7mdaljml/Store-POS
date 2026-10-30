use serde::{Deserialize, Serialize};
use sqlx::Row;
use tauri::{AppHandle, Runtime};

use super::guard;
use crate::db;

#[derive(Debug, Serialize)]
pub struct SettingEntry {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Serialize)]
pub struct TaxProfile {
    pub id: i64,
    pub name: String,
    pub rate: f64,
    pub is_default: i64,
}

#[derive(Debug, Serialize)]
pub struct Currency {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub symbol: String,
    pub rate: f64,
    pub is_base: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CustomerLite {
    pub id: i64,
    pub name: String,
    pub phone: Option<String>,
    pub balance: f64,
}

// ----------------------------- app settings -----------------------------

/// All key/value application settings. Readable by any signed-in user so the
/// UI can render store name, currency, receipt layout, page size, etc.
#[tauri::command]
pub async fn list_settings<R: Runtime>(app: AppHandle<R>) -> Result<Vec<SettingEntry>, String> {
    guard::require(&app, None)?;
    let pool = db::pool(&app).await?;
    let rows = sqlx::query("SELECT key, value FROM settings ORDER BY key")
        .fetch_all(&pool)
        .await
        .map_err(|e| e.to_string())?;
    rows.into_iter()
        .map(|row| {
            Ok(SettingEntry {
                key: row.try_get("key").map_err(|e| e.to_string())?,
                value: row.try_get("value").map_err(|e| e.to_string())?,
            })
        })
        .collect()
}

/// Persists one application setting. Restricted to settings managers and
/// audited so configuration changes are attributable.
#[tauri::command]
pub async fn set_setting<R: Runtime>(
    app: AppHandle<R>,
    key: String,
    value: String,
) -> Result<(), String> {
    let actor = guard::require(&app, Some("settings.manage"))?;
    let key = key.trim().to_string();
    if key.is_empty() {
        return Err("Setting key is required".into());
    }
    let pool = db::pool(&app).await?;
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    sqlx::query(
        "INSERT INTO settings (key, value) VALUES (?, ?)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(&key)
    .bind(&value)
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    sqlx::query(
        "INSERT INTO audit_log (user_id, action, entity_type, entity_id, details)
         VALUES (?, 'settings.update', 'setting', NULL, ?)",
    )
    .bind(actor.user_id)
    .bind(format!("Updated setting \"{key}\""))
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(())
}

// ----------------------------- tax profiles -----------------------------

#[tauri::command]
pub async fn list_tax_profiles<R: Runtime>(app: AppHandle<R>) -> Result<Vec<TaxProfile>, String> {
    guard::require(&app, None)?;
    let pool = db::pool(&app).await?;
    let rows = sqlx::query("SELECT id, name, rate, is_default FROM tax_profiles ORDER BY is_default DESC, name")
        .fetch_all(&pool)
        .await
        .map_err(|e| e.to_string())?;
    rows.into_iter()
        .map(|row| {
            Ok(TaxProfile {
                id: row.try_get("id").map_err(|e| e.to_string())?,
                name: row.try_get("name").map_err(|e| e.to_string())?,
                rate: row.try_get("rate").map_err(|e| e.to_string())?,
                is_default: row.try_get("is_default").map_err(|e| e.to_string())?,
            })
        })
        .collect()
}

async fn clear_other_defaults(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    keep_id: i64,
) -> Result<(), String> {
    sqlx::query("UPDATE tax_profiles SET is_default = CASE WHEN id = ? THEN 1 ELSE 0 END")
        .bind(keep_id)
        .execute(&mut **tx)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Creates (id = None) or updates a tax profile. A profile flagged default
/// becomes the only default.
#[tauri::command]
pub async fn save_tax_profile<R: Runtime>(
    app: AppHandle<R>,
    id: Option<i64>,
    name: String,
    rate: f64,
    is_default: bool,
) -> Result<i64, String> {
    let actor = guard::require(&app, Some("settings.manage"))?;
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("Tax profile name is required".into());
    }
    if !rate.is_finite() || rate < 0.0 {
        return Err("Tax rate must be zero or greater".into());
    }
    let pool = db::pool(&app).await?;
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    let saved_id = match id {
        Some(pid) => {
            let result = sqlx::query("UPDATE tax_profiles SET name = ?, rate = ? WHERE id = ?")
                .bind(&name)
                .bind(rate)
                .bind(pid)
                .execute(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
            if result.rows_affected() == 0 {
                return Err("Tax profile not found".into());
            }
            pid
        }
        None => sqlx::query("INSERT INTO tax_profiles (name, rate, is_default) VALUES (?, ?, 0)")
            .bind(&name)
            .bind(rate)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?
            .last_insert_rowid(),
    };
    if is_default {
        clear_other_defaults(&mut tx, saved_id).await?;
    }
    sqlx::query(
        "INSERT INTO audit_log (user_id, action, entity_type, entity_id, details)
         VALUES (?, 'tax.save', 'tax_profile', ?, ?)",
    )
    .bind(actor.user_id)
    .bind(saved_id)
    .bind(format!("Saved tax profile \"{name}\" at {rate}%"))
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(saved_id)
}

#[tauri::command]
pub async fn set_default_tax_profile<R: Runtime>(app: AppHandle<R>, id: i64) -> Result<(), String> {
    guard::require(&app, Some("settings.manage"))?;
    let pool = db::pool(&app).await?;
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    let exists: Option<i64> = sqlx::query_scalar("SELECT id FROM tax_profiles WHERE id = ?")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    if exists.is_none() {
        return Err("Tax profile not found".into());
    }
    clear_other_defaults(&mut tx, id).await?;
    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn delete_tax_profile<R: Runtime>(app: AppHandle<R>, id: i64) -> Result<(), String> {
    let actor = guard::require(&app, Some("settings.manage"))?;
    let pool = db::pool(&app).await?;
    let used: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM products WHERE tax_profile_id = ?")
        .bind(id)
        .fetch_one(&pool)
        .await
        .map_err(|e| e.to_string())?;
    if used > 0 {
        return Err(format!(
            "This tax profile is used by {used} product(s) and cannot be deleted."
        ));
    }
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    let result = sqlx::query("DELETE FROM tax_profiles WHERE id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    if result.rows_affected() == 0 {
        return Err("Tax profile not found".into());
    }
    sqlx::query(
        "INSERT INTO audit_log (user_id, action, entity_type, entity_id, details)
         VALUES (?, 'tax.delete', 'tax_profile', ?, 'Deleted tax profile')",
    )
    .bind(actor.user_id)
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(())
}

// ----------------------------- currencies -----------------------------

#[tauri::command]
pub async fn list_currencies<R: Runtime>(app: AppHandle<R>) -> Result<Vec<Currency>, String> {
    guard::require(&app, None)?;
    let pool = db::pool(&app).await?;
    let rows = sqlx::query("SELECT id, code, name, symbol, rate, is_base FROM currencies ORDER BY is_base DESC, code")
        .fetch_all(&pool)
        .await
        .map_err(|e| e.to_string())?;
    rows.into_iter()
        .map(|row| {
            Ok(Currency {
                id: row.try_get("id").map_err(|e| e.to_string())?,
                code: row.try_get("code").map_err(|e| e.to_string())?,
                name: row.try_get("name").map_err(|e| e.to_string())?,
                symbol: row.try_get("symbol").map_err(|e| e.to_string())?,
                rate: row.try_get("rate").map_err(|e| e.to_string())?,
                is_base: row.try_get("is_base").map_err(|e| e.to_string())?,
            })
        })
        .collect()
}

#[tauri::command]
pub async fn save_currency<R: Runtime>(
    app: AppHandle<R>,
    id: Option<i64>,
    code: String,
    name: String,
    symbol: String,
    rate: f64,
) -> Result<i64, String> {
    let actor = guard::require(&app, Some("settings.manage"))?;
    let code = code.trim().to_uppercase();
    let name = name.trim().to_string();
    let symbol = symbol.trim().to_string();
    if code.is_empty() || name.is_empty() {
        return Err("Currency code and name are required".into());
    }
    if !rate.is_finite() || rate <= 0.0 {
        return Err("Exchange rate must be greater than zero".into());
    }
    let pool = db::pool(&app).await?;
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    let saved_id = match id {
        Some(cid) => {
            let result = sqlx::query(
                "UPDATE currencies SET code = ?, name = ?, symbol = ?, rate = ? WHERE id = ?",
            )
            .bind(&code)
            .bind(&name)
            .bind(&symbol)
            .bind(rate)
            .bind(cid)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
            if result.rows_affected() == 0 {
                return Err("Currency not found".into());
            }
            cid
        }
        None => sqlx::query(
            "INSERT INTO currencies (code, name, symbol, rate, is_base) VALUES (?, ?, ?, ?, 0)",
        )
        .bind(&code)
        .bind(&name)
        .bind(&symbol)
        .bind(rate)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?
        .last_insert_rowid(),
    };
    sqlx::query(
        "INSERT INTO audit_log (user_id, action, entity_type, entity_id, details)
         VALUES (?, 'currency.save', 'currency', ?, ?)",
    )
    .bind(actor.user_id)
    .bind(saved_id)
    .bind(format!("Saved currency {code}"))
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(saved_id)
}

#[tauri::command]
pub async fn set_base_currency<R: Runtime>(app: AppHandle<R>, id: i64) -> Result<(), String> {
    let actor = guard::require(&app, Some("settings.manage"))?;
    let pool = db::pool(&app).await?;
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    let row: Option<(String,)> = sqlx::query_as("SELECT code FROM currencies WHERE id = ?")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    let code = row.ok_or_else(|| "Currency not found".to_string())?.0;
    sqlx::query("UPDATE currencies SET is_base = CASE WHEN id = ? THEN 1 ELSE 0 END")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query(
        "INSERT INTO settings (key, value) VALUES ('currency', ?)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(&code)
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    sqlx::query(
        "INSERT INTO audit_log (user_id, action, entity_type, entity_id, details)
         VALUES (?, 'currency.base', 'currency', ?, ?)",
    )
    .bind(actor.user_id)
    .bind(id)
    .bind(format!("Set base currency to {code}"))
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn delete_currency<R: Runtime>(app: AppHandle<R>, id: i64) -> Result<(), String> {
    let actor = guard::require(&app, Some("settings.manage"))?;
    let pool = db::pool(&app).await?;
    let is_base: Option<i64> = sqlx::query_scalar("SELECT is_base FROM currencies WHERE id = ?")
        .bind(id)
        .fetch_optional(&pool)
        .await
        .map_err(|e| e.to_string())?;
    match is_base {
        Some(1) => return Err("The base currency cannot be deleted.".into()),
        Some(_) => {}
        None => return Err("Currency not found".into()),
    }
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    sqlx::query("DELETE FROM currencies WHERE id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query(
        "INSERT INTO audit_log (user_id, action, entity_type, entity_id, details)
         VALUES (?, 'currency.delete', 'currency', ?, 'Deleted currency')",
    )
    .bind(actor.user_id)
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(())
}

// --------------------------- lightweight lists ---------------------------

/// Customer name/id list used by the checkout, reports and filter dropdowns.
#[tauri::command]
pub async fn list_customers_lite<R: Runtime>(app: AppHandle<R>) -> Result<Vec<CustomerLite>, String> {
    guard::require(&app, None)?;
    let pool = db::pool(&app).await?;
    let rows = sqlx::query("SELECT id, name, phone, balance FROM customers ORDER BY name COLLATE NOCASE")
        .fetch_all(&pool)
        .await
        .map_err(|e| e.to_string())?;
    rows.into_iter()
        .map(|row| {
            Ok::<CustomerLite, String>(CustomerLite {
                id: row.try_get("id").map_err(|e| e.to_string())?,
                name: row.try_get("name").map_err(|e| e.to_string())?,
                phone: row.try_get("phone").map_err(|e| e.to_string())?,
                balance: row.try_get("balance").map_err(|e| e.to_string())?,
            })
        })
        .collect()
}
