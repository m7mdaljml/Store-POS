use serde::{Deserialize, Serialize};
use sqlx::Row;
use tauri::{AppHandle, Runtime};

use crate::db;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpenseCategoryRecord {
    pub id: i64,
    pub name: String,
    pub expense_count: i64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutgoingExpenseInput {
    pub category_id: Option<i64>,
    pub amount: f64,
    pub date: Option<String>,
    pub description: Option<String>,
    pub reference_no: Option<String>,
    pub user_id: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutgoingExpenseRecord {
    pub id: i64,
    pub category_id: Option<i64>,
    pub category_name: Option<String>,
    pub amount: f64,
    pub date: String,
    pub description: Option<String>,
    pub reference_no: Option<String>,
    pub user_name: Option<String>,
}

fn optional_field(value: &Option<String>) -> Option<String> {
    value.as_ref().map(|v| v.trim()).filter(|v| !v.is_empty()).map(String::from)
}

#[tauri::command]
pub async fn list_expense_categories<R: Runtime>(
    app: AppHandle<R>,
) -> Result<Vec<ExpenseCategoryRecord>, String> {
    let pool = db::pool(&app).await?;
    let rows = sqlx::query(
        "SELECT c.id, c.name, COUNT(e.id) AS expense_count
         FROM expense_categories c
         LEFT JOIN expense_out e ON e.category_id = c.id
         GROUP BY c.id
         ORDER BY c.name",
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| e.to_string())?;

    rows.into_iter()
        .map(|row| {
            Ok(ExpenseCategoryRecord {
                id: row.try_get("id").map_err(|e| e.to_string())?,
                name: row.try_get("name").map_err(|e| e.to_string())?,
                expense_count: row.try_get("expense_count").map_err(|e| e.to_string())?,
            })
        })
        .collect()
}

#[tauri::command]
pub async fn create_expense_category<R: Runtime>(
    app: AppHandle<R>,
    name: String,
) -> Result<i64, String> {
    let pool = db::pool(&app).await?;
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("Category name is required".into());
    }
    let exists: Option<(i64,)> = sqlx::query_as("SELECT id FROM expense_categories WHERE name = ?")
        .bind(&name)
        .fetch_optional(&pool)
        .await
        .map_err(|e| e.to_string())?;
    if exists.is_some() {
        return Err(format!("Category '{name}' already exists"));
    }
    let result = sqlx::query("INSERT INTO expense_categories (name) VALUES (?)")
        .bind(&name)
        .execute(&pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(result.last_insert_rowid())
}

#[tauri::command]
pub async fn delete_expense_category<R: Runtime>(
    app: AppHandle<R>,
    category_id: i64,
) -> Result<(), String> {
    let pool = db::pool(&app).await?;
    let used: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM expense_out WHERE category_id = ?")
        .bind(category_id)
        .fetch_one(&pool)
        .await
        .map_err(|e| e.to_string())?;
    if used.0 > 0 {
        return Err("This category has expenses recorded and cannot be deleted.".into());
    }
    let result = sqlx::query("DELETE FROM expense_categories WHERE id = ?")
        .bind(category_id)
        .execute(&pool)
        .await
        .map_err(|e| e.to_string())?;
    if result.rows_affected() == 0 {
        return Err("Category not found".into());
    }
    Ok(())
}

#[tauri::command]
pub async fn list_expenses_out<R: Runtime>(
    app: AppHandle<R>,
) -> Result<Vec<OutgoingExpenseRecord>, String> {
    let pool = db::pool(&app).await?;
    let rows = sqlx::query(
        "SELECT e.id, e.category_id, c.name, e.amount, e.date, e.description, e.reference_no, u.full_name
         FROM expense_out e
         LEFT JOIN expense_categories c ON c.id = e.category_id
         LEFT JOIN users u ON u.id = e.user_id
         ORDER BY e.date DESC, e.id DESC",
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| e.to_string())?;

    rows.into_iter()
        .map(|row| {
            Ok(OutgoingExpenseRecord {
                id: row.try_get("id").map_err(|e| e.to_string())?,
                category_id: row.try_get("category_id").map_err(|e| e.to_string())?,
                category_name: row.try_get("category_name").map_err(|e| e.to_string())?,
                amount: row.try_get("amount").map_err(|e| e.to_string())?,
                date: row.try_get("date").map_err(|e| e.to_string())?,
                description: row.try_get("description").map_err(|e| e.to_string())?,
                reference_no: row.try_get("reference_no").map_err(|e| e.to_string())?,
                user_name: row.try_get("user_name").map_err(|e| e.to_string())?,
            })
        })
        .collect()
}

pub async fn create_outgoing_expense(
    pool: &sqlx::SqlitePool,
    input: OutgoingExpenseInput,
) -> Result<i64, String> {
    if input.amount <= 0.0 {
        return Err("Amount must be greater than zero".into());
    }
    if let Some(category_id) = input.category_id {
        let exists: Option<(i64,)> =
            sqlx::query_as("SELECT id FROM expense_categories WHERE id = ?")
                .bind(category_id)
                .fetch_optional(pool)
                .await
                .map_err(|e| e.to_string())?;
        if exists.is_none() {
            return Err("Category not found".into());
        }
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
        "INSERT INTO expense_out (category_id, amount, date, description, reference_no, user_id)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(input.category_id)
    .bind(input.amount)
    .bind(&date)
    .bind(optional_field(&input.description))
    .bind(optional_field(&input.reference_no))
    .bind(input.user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    let expense_id = result.last_insert_rowid();

    sqlx::query(
        "INSERT INTO audit_log (user_id, action, entity_type, entity_id, details)
         VALUES (?, 'expense.outgoing', 'expense_out', ?, ?)",
    )
    .bind(input.user_id)
    .bind(expense_id)
    .bind(format!("Recorded outgoing expense of {}", input.amount))
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(expense_id)
}

#[tauri::command]
pub async fn add_expense_out<R: Runtime>(
    app: AppHandle<R>,
    input: OutgoingExpenseInput,
) -> Result<i64, String> {
    let pool = db::pool(&app).await?;
    create_outgoing_expense(&pool, input).await
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::db;

    pub(crate) async fn create_schema(path: &std::path::Path) {
        let pool = db::connect(path).await.unwrap();
        for sql in [
            "CREATE TABLE expense_categories (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT UNIQUE NOT NULL)",
            "CREATE TABLE expense_out (id INTEGER PRIMARY KEY AUTOINCREMENT, category_id INTEGER, amount REAL NOT NULL, date TEXT NOT NULL DEFAULT (datetime('now')), description TEXT, reference_no TEXT, user_id INTEGER, created_at TEXT NOT NULL DEFAULT (datetime('now')))",
            "CREATE TABLE audit_log (id INTEGER PRIMARY KEY AUTOINCREMENT, user_id INTEGER, action TEXT NOT NULL, entity_type TEXT, entity_id INTEGER, details TEXT, created_at TEXT NOT NULL DEFAULT (datetime('now')))",
        ] {
            sqlx::query(sql).execute(&pool).await.unwrap();
        }
        pool.close().await;
    }

    fn temp_db(name: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "store-pos-{name}-{}-{nanos}.db",
            std::process::id()
        ))
    }

    #[tokio::test]
    async fn create_outgoing_expense_records_and_audits() {
        let path = temp_db("expense-valid");
        create_schema(&path).await;
        let pool = db::connect(&path).await.unwrap();

        let cat_id = sqlx::query("INSERT INTO expense_categories (name) VALUES ('Rent')")
            .execute(&pool)
            .await
            .unwrap()
            .last_insert_rowid();

        let id = create_outgoing_expense(
            &pool,
            OutgoingExpenseInput {
                category_id: Some(cat_id),
                amount: 150.0,
                date: Some("2026-08-15".into()),
                description: Some("August rent".into()),
                reference_no: Some("REF-01".into()),
                user_id: None,
            },
        )
        .await
        .unwrap();

        let (amount, category_id, description, reference_no): (f64, Option<i64>, Option<String>, Option<String>) =
            sqlx::query_as("SELECT amount, category_id, description, reference_no FROM expense_out WHERE id = ?")
                .bind(id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(amount, 150.0);
        assert_eq!(category_id, Some(cat_id));
        assert_eq!(description.unwrap(), "August rent");
        assert_eq!(reference_no.unwrap(), "REF-01");

        let audit: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM audit_log WHERE action = 'expense.outgoing' AND entity_id = ?",
        )
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(audit, 1);

        pool.close().await;
        std::fs::remove_file(&path).ok();
    }

    #[tokio::test]
    async fn create_outgoing_expense_rejects_invalid_input() {
        let path = temp_db("expense-invalid");
        create_schema(&path).await;
        let pool = db::connect(&path).await.unwrap();

        let zero_amount = create_outgoing_expense(
            &pool,
            OutgoingExpenseInput {
                category_id: None,
                amount: 0.0,
                date: None,
                description: None,
                reference_no: None,
                user_id: None,
            },
        )
        .await;
        assert!(zero_amount.is_err());

        let missing_category = create_outgoing_expense(
            &pool,
            OutgoingExpenseInput {
                category_id: Some(9999),
                amount: 10.0,
                date: None,
                description: None,
                reference_no: None,
                user_id: None,
            },
        )
        .await;
        assert!(missing_category.is_err());

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM expense_out")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 0, "no partial expense rows after failures");

        pool.close().await;
        std::fs::remove_file(&path).ok();
    }
}
