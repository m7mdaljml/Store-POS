use serde::Serialize;
use sqlx::Row;
use tauri::{AppHandle, Runtime};

use super::{guard, Page};
use crate::db;

/// Records a debt payment for a customer: decreases `customers.balance`
/// and appends a 'payment' entry to `customer_ledger`. Returns the new balance.
async fn record_payment(
    pool: &sqlx::SqlitePool,
    customer_id: i64,
    amount: f64,
    notes: Option<String>,
    user_id: Option<i64>,
) -> Result<f64, String> {
    if !amount.is_finite() || amount <= 0.0 {
        return Err("Payment amount must be greater than zero".into());
    }
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    let current: (f64,) = sqlx::query_as("SELECT balance FROM customers WHERE id = ?")
        .bind(customer_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Customer not found".to_string())?;

    if amount > current.0 + 0.005 {
        return Err("Amount exceeds the customer's outstanding balance".into());
    }

    let balance_after = current.0 - amount;
    sqlx::query("UPDATE customers SET balance = ? WHERE id = ?")
        .bind(balance_after)
        .bind(customer_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    sqlx::query(
        "INSERT INTO customer_ledger (customer_id, sale_id, type, amount, balance_after, notes, user_id)
         VALUES (?, NULL, 'payment', ?, ?, ?, ?)",
    )
    .bind(customer_id)
    .bind(amount)
    .bind(balance_after)
    .bind(notes.as_deref().map(str::trim).filter(|s| !s.is_empty()))
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query(
        "INSERT INTO audit_log (user_id, action, entity_type, entity_id, details)
         VALUES (?, 'customer.payment', 'customer', ?, ?)",
    )
    .bind(user_id)
    .bind(customer_id)
    .bind(format!(
        "Customer payment of {}",
        crate::format::money(amount)
    ))
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(balance_after)
}

#[tauri::command]
pub async fn record_customer_payment<R: Runtime>(
    app: AppHandle<R>,
    customer_id: i64,
    amount: f64,
    notes: Option<String>,
    user_id: Option<i64>,
) -> Result<f64, String> {
    let actor = guard::require(&app, Some("expenses.manage"))?;
    let _ = user_id;
    let pool = db::pool(&app).await?;
    record_payment(&pool, customer_id, amount, notes, Some(actor.user_id)).await
}

#[derive(Debug, Serialize)]
pub struct CustomerRecord {
    pub id: i64,
    pub name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub balance: f64,
    pub notes: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct CustomerStats {
    pub total: i64,
    pub debtors_count: i64,
    pub total_debt: f64,
}

#[derive(Debug, Serialize)]
pub struct CustomerSaleRow {
    pub id: i64,
    pub sale_no: String,
    pub created_at: String,
    pub net_total: f64,
    pub paid_amount: f64,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct CustomerLedgerRow {
    pub id: i64,
    pub r#type: String,
    pub amount: f64,
    pub balance_after: f64,
    pub notes: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct CustomerHistory {
    pub sales: Vec<CustomerSaleRow>,
    pub ledger: Vec<CustomerLedgerRow>,
}

#[derive(Debug, Serialize)]
pub struct CustomerDeleteCheck {
    pub balance: f64,
    pub sale_count: i64,
}

fn map_customer(row: &sqlx::sqlite::SqliteRow) -> Result<CustomerRecord, String> {
    Ok(CustomerRecord {
        id: row.try_get("id").map_err(|e| e.to_string())?,
        name: row.try_get("name").map_err(|e| e.to_string())?,
        phone: row.try_get("phone").map_err(|e| e.to_string())?,
        email: row.try_get("email").map_err(|e| e.to_string())?,
        address: row.try_get("address").map_err(|e| e.to_string())?,
        balance: row.try_get("balance").map_err(|e| e.to_string())?,
        notes: row.try_get("notes").map_err(|e| e.to_string())?,
        created_at: row.try_get("created_at").map_err(|e| e.to_string())?,
    })
}

/// Paged, searchable customer list for the customers screen.
#[tauri::command]
pub async fn list_customers<R: Runtime>(
    app: AppHandle<R>,
    search: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Page<CustomerRecord>, String> {
    guard::require(&app, Some("expenses.manage"))?;
    let pool = db::pool(&app).await?;
    let limit = limit.unwrap_or(50).max(1).min(500);
    let offset = offset.unwrap_or(0).max(0);

    let pattern = search
        .as_deref()
        .map(|s| format!("%{}%", s.trim()))
        .filter(|p| p != "%%");
    let where_clause = if pattern.is_some() {
        "WHERE name LIKE ? OR IFNULL(phone,'') LIKE ? OR IFNULL(email,'') LIKE ?
                OR IFNULL(address,'') LIKE ? OR IFNULL(notes,'') LIKE ?"
    } else {
        ""
    };

    let sql = format!(
        "SELECT id, name, phone, email, address, balance, notes, created_at
         FROM customers {where_clause}
         ORDER BY name COLLATE NOCASE LIMIT ? OFFSET ?"
    );
    let mut query = sqlx::query(&sql);
    if let Some(p) = &pattern {
        query = query
            .bind(p)
            .bind(p)
            .bind(p)
            .bind(p)
            .bind(p);
    }
    let rows = query
        .bind(limit)
        .bind(offset)
        .fetch_all(&pool)
        .await
        .map_err(|e| e.to_string())?;
    let items: Vec<CustomerRecord> = rows.iter().map(map_customer).collect::<Result<_, _>>()?;

    let count_sql = format!("SELECT COUNT(*) AS total FROM customers {where_clause}");
    let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
    if let Some(p) = &pattern {
        count_query = count_query
            .bind(p)
            .bind(p)
            .bind(p)
            .bind(p)
            .bind(p);
    }
    let total = count_query
        .fetch_one(&pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(Page { items, total })
}

/// Aggregate KPIs: total customers, number in debt and total outstanding debt.
#[tauri::command]
pub async fn customer_stats<R: Runtime>(app: AppHandle<R>) -> Result<CustomerStats, String> {
    guard::require(&app, Some("expenses.manage"))?;
    let pool = db::pool(&app).await?;
    customer_stats_values(&pool).await
}

/// Shared stats query. Uses `TOTAL()` (always REAL) rather than
/// `COALESCE(SUM(...), 0)`, which returns an INTEGER on an empty table or when
/// no customer is in debt, and then fails to decode into `f64`.
async fn customer_stats_values(pool: &sqlx::SqlitePool) -> Result<CustomerStats, String> {
    let row = sqlx::query(
        "SELECT COUNT(*) AS total,
                SUM(CASE WHEN balance > 0.005 THEN 1 ELSE 0 END) AS debtors_count,
                TOTAL(CASE WHEN balance > 0.005 THEN balance ELSE 0.0 END) AS total_debt
         FROM customers",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(CustomerStats {
        total: row.try_get("total").map_err(|e| e.to_string())?,
        debtors_count: row
            .try_get::<Option<i64>, _>("debtors_count")
            .map_err(|e| e.to_string())?
            .unwrap_or(0),
        total_debt: row.try_get("total_debt").map_err(|e| e.to_string())?,
    })
}

/// Customers with an outstanding balance, largest debt first.
#[tauri::command]
pub async fn list_debtors<R: Runtime>(
    app: AppHandle<R>,
    limit: Option<i64>,
) -> Result<Vec<CustomerRecord>, String> {
    guard::require(&app, Some("expenses.manage"))?;
    let pool = db::pool(&app).await?;
    let limit = limit.unwrap_or(50).max(1).min(500);
    let rows = sqlx::query(
        "SELECT id, name, phone, email, address, balance, notes, created_at
         FROM customers WHERE balance > 0.005 ORDER BY balance DESC LIMIT ?",
    )
    .bind(limit)
    .fetch_all(&pool)
    .await
    .map_err(|e| e.to_string())?;
    rows.iter().map(map_customer).collect()
}

/// Sales history and ledger entries for a customer's detail panel.
#[tauri::command]
pub async fn get_customer_history<R: Runtime>(
    app: AppHandle<R>,
    customer_id: i64,
) -> Result<CustomerHistory, String> {
    guard::require(&app, Some("expenses.manage"))?;
    let pool = db::pool(&app).await?;

    let sale_rows = sqlx::query(
        "SELECT id, sale_no, created_at,
                total - COALESCE((SELECT TOTAL(amount) FROM refunds WHERE sale_id = sales.id), 0.0) AS net_total,
                paid_amount, status
         FROM sales WHERE customer_id = ? ORDER BY id DESC LIMIT 50",
    )
    .bind(customer_id)
    .fetch_all(&pool)
    .await
    .map_err(|e| e.to_string())?;

    let sales: Vec<CustomerSaleRow> = sale_rows
        .iter()
        .map(|row| {
            Ok::<CustomerSaleRow, String>(CustomerSaleRow {
                id: row.try_get("id").map_err(|e| e.to_string())?,
                sale_no: row.try_get("sale_no").map_err(|e| e.to_string())?,
                created_at: row.try_get("created_at").map_err(|e| e.to_string())?,
                net_total: row.try_get("net_total").map_err(|e| e.to_string())?,
                paid_amount: row.try_get("paid_amount").map_err(|e| e.to_string())?,
                status: row.try_get("status").map_err(|e| e.to_string())?,
            })
        })
        .collect::<Result<_, _>>()?;

    let ledger_rows = sqlx::query(
        "SELECT id, type, amount, balance_after, notes, created_at
         FROM customer_ledger WHERE customer_id = ? ORDER BY id DESC LIMIT 100",
    )
    .bind(customer_id)
    .fetch_all(&pool)
    .await
    .map_err(|e| e.to_string())?;

    let ledger: Vec<CustomerLedgerRow> = ledger_rows
        .iter()
        .map(|row| {
            Ok::<CustomerLedgerRow, String>(CustomerLedgerRow {
                id: row.try_get("id").map_err(|e| e.to_string())?,
                r#type: row.try_get("type").map_err(|e| e.to_string())?,
                amount: row.try_get("amount").map_err(|e| e.to_string())?,
                balance_after: row.try_get("balance_after").map_err(|e| e.to_string())?,
                notes: row.try_get("notes").map_err(|e| e.to_string())?,
                created_at: row.try_get("created_at").map_err(|e| e.to_string())?,
            })
        })
        .collect::<Result<_, _>>()?;

    Ok(CustomerHistory { sales, ledger })
}

/// Pre-delete check so the UI can block deleting a customer with debt or sales
/// history and show a translated message.
#[tauri::command]
pub async fn customer_delete_check<R: Runtime>(
    app: AppHandle<R>,
    customer_id: i64,
) -> Result<CustomerDeleteCheck, String> {
    guard::require(&app, Some("expenses.manage"))?;
    let pool = db::pool(&app).await?;
    let row = sqlx::query(
        "SELECT balance, (SELECT COUNT(*) FROM sales WHERE customer_id = ?) AS sale_count
         FROM customers WHERE id = ?",
    )
    .bind(customer_id)
    .bind(customer_id)
    .fetch_optional(&pool)
    .await
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "Customer not found".to_string())?;
    Ok(CustomerDeleteCheck {
        balance: row.try_get("balance").map_err(|e| e.to_string())?,
        sale_count: row.try_get("sale_count").map_err(|e| e.to_string())?,
    })
}

fn clean(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[tauri::command]
pub async fn create_customer<R: Runtime>(
    app: AppHandle<R>,
    name: String,
    phone: String,
    email: String,
    address: String,
    notes: String,
) -> Result<i64, String> {
    let actor = guard::require(&app, Some("expenses.manage"))?;
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("Customer name is required".into());
    }
    let pool = db::pool(&app).await?;
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    let result = sqlx::query(
        "INSERT INTO customers (name, phone, email, address, notes) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&name)
    .bind(clean(&phone))
    .bind(clean(&email))
    .bind(clean(&address))
    .bind(clean(&notes))
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    let id = result.last_insert_rowid();
    sqlx::query(
        "INSERT INTO audit_log (user_id, action, entity_type, entity_id, details)
         VALUES (?, 'customer.create', 'customer', ?, ?)",
    )
    .bind(actor.user_id)
    .bind(id)
    .bind(format!("Created customer \"{name}\""))
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
pub async fn update_customer<R: Runtime>(
    app: AppHandle<R>,
    customer_id: i64,
    name: String,
    phone: String,
    email: String,
    address: String,
    notes: String,
) -> Result<(), String> {
    let actor = guard::require(&app, Some("expenses.manage"))?;
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("Customer name is required".into());
    }
    let pool = db::pool(&app).await?;
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    let result = sqlx::query(
        "UPDATE customers SET name = ?, phone = ?, email = ?, address = ?, notes = ? WHERE id = ?",
    )
    .bind(&name)
    .bind(clean(&phone))
    .bind(clean(&email))
    .bind(clean(&address))
    .bind(clean(&notes))
    .bind(customer_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    if result.rows_affected() == 0 {
        return Err("Customer not found".into());
    }
    sqlx::query(
        "INSERT INTO audit_log (user_id, action, entity_type, entity_id, details)
         VALUES (?, 'customer.update', 'customer', ?, ?)",
    )
    .bind(actor.user_id)
    .bind(customer_id)
    .bind(format!("Updated customer \"{name}\""))
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(())
}

/// Deletes a customer. Enforces the same rules the UI shows: no outstanding
/// balance and no sales history. Ledger rows cascade.
#[tauri::command]
pub async fn delete_customer<R: Runtime>(
    app: AppHandle<R>,
    customer_id: i64,
) -> Result<(), String> {
    let actor = guard::require(&app, Some("expenses.manage"))?;
    let pool = db::pool(&app).await?;
    let row = sqlx::query(
        "SELECT name, balance, (SELECT COUNT(*) FROM sales WHERE customer_id = ?) AS sale_count
         FROM customers WHERE id = ?",
    )
    .bind(customer_id)
    .bind(customer_id)
    .fetch_optional(&pool)
    .await
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "Customer not found".to_string())?;
    let name: String = row.try_get("name").map_err(|e| e.to_string())?;
    let balance: f64 = row.try_get("balance").map_err(|e| e.to_string())?;
    let sale_count: i64 = row.try_get("sale_count").map_err(|e| e.to_string())?;
    if balance > 0.005 {
        return Err("This customer has an outstanding balance and cannot be deleted.".into());
    }
    if sale_count > 0 {
        return Err(format!(
            "This customer has {sale_count} sale(s) and cannot be deleted."
        ));
    }

    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    sqlx::query("DELETE FROM customers WHERE id = ?")
        .bind(customer_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query(
        "INSERT INTO audit_log (user_id, action, entity_type, entity_id, details)
         VALUES (?, 'customer.delete', 'customer', ?, ?)",
    )
    .bind(actor.user_id)
    .bind(customer_id)
    .bind(format!("Deleted customer \"{name}\""))
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schema() -> &'static str {
        r#"
        CREATE TABLE customers (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          name TEXT NOT NULL,
          phone TEXT,
          email TEXT,
          address TEXT,
          balance REAL NOT NULL DEFAULT 0,
          notes TEXT,
          created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE TABLE customer_ledger (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          customer_id INTEGER NOT NULL REFERENCES customers(id) ON DELETE CASCADE,
          sale_id INTEGER,
          type TEXT NOT NULL,
          amount REAL NOT NULL,
          balance_after REAL NOT NULL,
          notes TEXT,
          user_id INTEGER,
          created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE TABLE audit_log (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          user_id INTEGER,
          action TEXT NOT NULL,
          entity_type TEXT,
          entity_id INTEGER,
          details TEXT,
          created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        "#
    }

    async fn mem_pool() -> sqlx::SqlitePool {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::raw_sql(schema()).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO customers (id, name, balance) VALUES (1, 'Omar', 25.0)")
            .execute(&pool)
            .await
            .unwrap();
        pool
    }

    #[tokio::test]
    async fn payment_reduces_balance_and_writes_ledger() {
        let pool = mem_pool().await;

        let new_balance = record_payment(&pool, 1, 10.0, Some("cash".into()), Some(3))
            .await
            .unwrap();

        assert!((new_balance - 15.0).abs() < 0.001);
        let balance: (f64,) = sqlx::query_as("SELECT balance FROM customers WHERE id = 1")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!((balance.0 - 15.0).abs() < 0.001);

        let entry: (String, f64, f64, Option<String>, i64) =
            sqlx::query_as(
                "SELECT type, amount, balance_after, notes, user_id FROM customer_ledger WHERE customer_id = 1",
            )
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(entry.0, "payment");
        assert!((entry.1 - 10.0).abs() < 0.001);
        assert!((entry.2 - 15.0).abs() < 0.001);
        assert_eq!(entry.3.as_deref(), Some("cash"));
        assert_eq!(entry.4, 3);
    }

    #[tokio::test]
    async fn payment_validates_amount_and_balance() {
        let pool = mem_pool().await;

        // Zero / negative / non-finite amounts are rejected.
        assert!(record_payment(&pool, 1, 0.0, None, None).await.is_err());
        assert!(record_payment(&pool, 1, -5.0, None, None).await.is_err());

        // Overpayment is rejected and changes nothing.
        assert!(record_payment(&pool, 1, 30.0, None, None).await.is_err());
        let balance: (f64,) = sqlx::query_as("SELECT balance FROM customers WHERE id = 1")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!((balance.0 - 25.0).abs() < 0.001);
        let count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM customer_ledger").fetch_one(&pool).await.unwrap();
        assert_eq!(count.0, 0);

        // Unknown customer is rejected.
        assert!(record_payment(&pool, 99, 5.0, None, None).await.is_err());
    }

    #[tokio::test]
    async fn stats_decode_as_real_without_debtors() {
        // Regression: `COALESCE(SUM(...), 0)` returned an INTEGER (0) on an empty
        // customers table or when no balance was positive, so sqlx refused to
        // decode "total_debt" into f64. `TOTAL()` always yields REAL.
        let pool = mem_pool().await;
        sqlx::query("DELETE FROM customers").execute(&pool).await.unwrap();

        let empty = customer_stats_values(&pool).await.unwrap();
        assert_eq!(empty.total, 0);
        assert_eq!(empty.debtors_count, 0);
        assert!(empty.total_debt.abs() < 0.001);

        // A customer with no debt exercises the CASE ... ELSE branch (integer 0).
        sqlx::query("INSERT INTO customers (id, name, balance) VALUES (2, 'Sara', 0)")
            .execute(&pool)
            .await
            .unwrap();
        let no_debt = customer_stats_values(&pool).await.unwrap();
        assert_eq!(no_debt.total, 1);
        assert_eq!(no_debt.debtors_count, 0);
        assert!(no_debt.total_debt.abs() < 0.001);

        // With a debtor the total still decodes as REAL.
        sqlx::query("INSERT INTO customers (id, name, balance) VALUES (3, 'Ali', 12.5)")
            .execute(&pool)
            .await
            .unwrap();
        let with_debt = customer_stats_values(&pool).await.unwrap();
        assert_eq!(with_debt.debtors_count, 1);
        assert!((with_debt.total_debt - 12.5).abs() < 0.001);
    }
}
