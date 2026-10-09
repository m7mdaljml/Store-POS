use serde::{Deserialize, Serialize};
use sqlx::Row;
use tauri::{AppHandle, Runtime};

use super::Page;
use crate::db;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CashWithdrawalInput {
    pub session_id: Option<i64>,
    pub user_id: i64,
    pub amount: f64,
    pub date: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CashWithdrawalRecord {
    pub id: i64,
    pub session_id: Option<i64>,
    pub user_id: Option<i64>,
    pub user_name: Option<String>,
    pub amount: f64,
    pub date: String,
    pub notes: Option<String>,
    pub created_at: String,
}

fn optional_field(value: &Option<String>) -> Option<String> {
    value.as_ref().map(|v| v.trim()).filter(|v| !v.is_empty()).map(String::from)
}

pub async fn create_cash_withdrawal(
    pool: &sqlx::SqlitePool,
    input: CashWithdrawalInput,
) -> Result<i64, String> {
    if input.amount <= 0.0 {
        return Err("Amount must be greater than zero".into());
    }

    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    let date = match optional_field(&input.date) {
        Some(d) => d,
        None => {
            let d: (String,) = sqlx::query_as("SELECT date('now')")
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
            d.0
        }
    };

    let result = sqlx::query(
        "INSERT INTO cash_withdrawals (session_id, user_id, amount, date, notes)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(input.session_id)
    .bind(input.user_id)
    .bind(input.amount)
    .bind(&date)
    .bind(optional_field(&input.notes))
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    let withdrawal_id = result.last_insert_rowid();

    sqlx::query(
        "INSERT INTO audit_log (user_id, action, entity_type, entity_id, details)
         VALUES (?, 'cash.withdrawal', 'cash_withdrawal', ?, ?)",
    )
    .bind(input.user_id)
    .bind(withdrawal_id)
    .bind(format!(
        "Recorded cash withdrawal of {}",
        crate::format::money(input.amount)
    ))
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(withdrawal_id)
}

#[tauri::command]
pub async fn add_cash_withdrawal<R: Runtime>(
    app: AppHandle<R>,
    input: CashWithdrawalInput,
) -> Result<i64, String> {
    let pool = db::pool(&app).await?;
    create_cash_withdrawal(&pool, input).await
}

#[tauri::command]
pub async fn list_cash_withdrawals<R: Runtime>(
    app: AppHandle<R>,
    session_id: Option<i64>,
    user_id: Option<i64>,
    from: Option<String>,
    to: Option<String>,
    search: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Page<CashWithdrawalRecord>, String> {
    let pool = db::pool(&app).await?;
    let limit = limit.unwrap_or(100).max(1).min(500);
    let offset = offset.unwrap_or(0).max(0);

    let mut conditions = Vec::new();
    let mut bind_i64s: Vec<Option<i64>> = Vec::new();
    let mut bind_values: Vec<Option<String>> = Vec::new();

    if session_id.is_some() {
        conditions.push("cw.session_id = ?");
        bind_i64s.push(session_id);
    }
    if user_id.is_some() {
        conditions.push("cw.user_id = ?");
        bind_i64s.push(user_id);
    }
    if let Some(f) = &from {
        conditions.push("date(cw.date) >= date(?)");
        bind_values.push(Some(f.clone()));
    }
    if let Some(t) = &to {
        conditions.push("date(cw.date) <= date(?)");
        bind_values.push(Some(t.clone()));
    }
    if let Some(s) = &search {
        let p = format!("%{}%", s.trim());
        if p != "%%" {
            conditions.push("(cw.notes LIKE ? OR COALESCE(u.full_name, '') LIKE ? OR COALESCE(u.username, '') LIKE ?)");
            bind_values.push(Some(p.clone()));
            bind_values.push(Some(p.clone()));
            bind_values.push(Some(p.clone()));
        }
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let sql = format!(
        "SELECT cw.id, cw.session_id, cw.user_id, u.full_name AS user_name, cw.amount, cw.date, cw.notes, cw.created_at
         FROM cash_withdrawals cw
         LEFT JOIN users u ON u.id = cw.user_id
         {where_clause}
         ORDER BY cw.date DESC, cw.created_at DESC, cw.id DESC
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

    let items: Vec<CashWithdrawalRecord> = rows
        .into_iter()
        .map(|row| {
            Ok(CashWithdrawalRecord {
                id: row.try_get("id").map_err(|e| e.to_string())?,
                session_id: row.try_get("session_id").map_err(|e| e.to_string())?,
                user_id: row.try_get("user_id").map_err(|e| e.to_string())?,
                user_name: row.try_get("user_name").map_err(|e| e.to_string())?,
                amount: row.try_get("amount").map_err(|e| e.to_string())?,
                date: row.try_get("date").map_err(|e| e.to_string())?,
                notes: row.try_get("notes").map_err(|e| e.to_string())?,
                created_at: row.try_get("created_at").map_err(|e| e.to_string())?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    let count_sql = format!(
        "SELECT COUNT(*) FROM cash_withdrawals cw
         LEFT JOIN users u ON u.id = cw.user_id
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
#[tauri::command]
pub async fn export_cash_withdrawals<R: Runtime>(
    app: AppHandle<R>,
    path: String,
    from: Option<String>,
    to: Option<String>,
    user_id: Option<i64>,
) -> Result<(), String> {
    let page = list_cash_withdrawals(app.clone(), None, user_id, from, to, None, Some(100000), Some(0)).await?;
    let headers = ["ID", "Date", "Amount", "User", "Notes", "Session"];
    let rows: Vec<Vec<String>> = page
        .items
        .iter()
        .map(|r| {
            vec![
                r.id.to_string(),
                r.date.clone(),
                format!("{:.2}", r.amount),
                r.user_name.clone().unwrap_or_default(),
                r.notes.clone().unwrap_or_default(),
                r.session_id.map(|s| s.to_string()).unwrap_or_default(),
            ]
        })
        .collect();
    crate::export::write_xlsx(std::path::Path::new(&path), "Cash Withdrawals", &headers, &rows)
}

#[tauri::command]
pub async fn delete_cash_withdrawal<R: Runtime>(
    app: AppHandle<R>,
    withdrawal_id: i64,
    user_id: Option<i64>,
) -> Result<(), String> {
    let pool = db::pool(&app).await?;

    let amount_row: Option<(f64,)> =
        sqlx::query_as("SELECT amount FROM cash_withdrawals WHERE id = ?")
            .bind(withdrawal_id)
            .fetch_optional(&pool)
            .await
            .map_err(|e| e.to_string())?;
    let amount = match amount_row {
        Some((a,)) => a,
        None => return Err("Withdrawal not found".into()),
    };

    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    let result = sqlx::query("DELETE FROM cash_withdrawals WHERE id = ?")
        .bind(withdrawal_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    if result.rows_affected() == 0 {
        return Err("Withdrawal not found".into());
    }

    sqlx::query(
        "INSERT INTO audit_log (user_id, action, entity_type, entity_id, details)
         VALUES (?, 'cash.withdrawal.delete', 'cash_withdrawal', ?, ?)",
    )
    .bind(user_id)
    .bind(withdrawal_id)
    .bind(format!(
        "Deleted cash withdrawal of {}",
        crate::format::money(amount)
    ))
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(())
}
