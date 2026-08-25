use serde::Serialize;
use sqlx::Row;
use tauri::{AppHandle, Runtime};

use super::Page;
use crate::db;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditLogEntry {
    pub id: i64,
    pub user_id: Option<i64>,
    pub user_name: Option<String>,
    pub action: String,
    pub entity_type: Option<String>,
    pub entity_id: Option<i64>,
    pub details: Option<String>,
    pub created_at: String,
}

#[tauri::command]
pub async fn list_audit_logs<R: Runtime>(
    app: AppHandle<R>,
    user_id: Option<i64>,
    action: Option<String>,
    search: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Page<AuditLogEntry>, String> {
    let pool = db::pool(&app).await?;
    let limit = limit.unwrap_or(100).max(1).min(500);
    let offset = offset.unwrap_or(0).max(0);

    let pattern = search
        .as_deref()
        .map(|s| format!("%{}%", s.trim()))
        .filter(|p| p != "%%");

    let mut conditions = Vec::new();
    let mut bind_values: Vec<Option<String>> = Vec::new();
    let mut bind_i64s: Vec<Option<i64>> = Vec::new();

    if user_id.is_some() {
        conditions.push("al.user_id = ?");
        bind_i64s.push(user_id);
    }
    if action.is_some() {
        conditions.push("al.action = ?");
        bind_values.push(action.clone());
    }
    if pattern.is_some() {
        conditions.push("(al.details LIKE ? OR al.action LIKE ? OR COALESCE(u.username, '') LIKE ? OR COALESCE(u.full_name, '') LIKE ?)");
        bind_values.push(pattern.clone());
        bind_values.push(pattern.clone());
        bind_values.push(pattern.clone());
        bind_values.push(pattern.clone());
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let sql = format!(
        "SELECT al.id, al.user_id, u.full_name AS user_name, al.action,
                al.entity_type, al.entity_id, al.details, al.created_at
         FROM audit_log al
         LEFT JOIN users u ON u.id = al.user_id
         {where_clause}
         ORDER BY al.created_at DESC, al.id DESC
         LIMIT ? OFFSET ?"
    );

    let mut query = sqlx::query(&sql);
    for v in &bind_i64s {
        query = query.bind(*v);
    }
    for v in &bind_values {
        query = query.bind(v.as_deref());
    }
    query = query.bind(limit).bind(offset);

    let rows = query
        .fetch_all(&pool)
        .await
        .map_err(|e| e.to_string())?;

    let items: Vec<AuditLogEntry> = rows
        .into_iter()
        .map(|row| {
            Ok(AuditLogEntry {
                id: row.try_get("id").map_err(|e| e.to_string())?,
                user_id: row.try_get("user_id").map_err(|e| e.to_string())?,
                user_name: row.try_get("user_name").map_err(|e| e.to_string())?,
                action: row.try_get("action").map_err(|e| e.to_string())?,
                entity_type: row.try_get("entity_type").map_err(|e| e.to_string())?,
                entity_id: row.try_get("entity_id").map_err(|e| e.to_string())?,
                details: row.try_get("details").map_err(|e| e.to_string())?,
                created_at: row.try_get("created_at").map_err(|e| e.to_string())?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    let count_sql = format!(
        "SELECT COUNT(*) FROM audit_log al
         LEFT JOIN users u ON u.id = al.user_id
         {where_clause}"
    );
    let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
    for v in &bind_i64s {
        count_query = count_query.bind(*v);
    }
    for v in &bind_values {
        count_query = count_query.bind(v.as_deref());
    }
    let total = count_query
        .fetch_one(&pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(Page { items, total })
}
