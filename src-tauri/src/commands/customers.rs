use tauri::{AppHandle, Runtime};

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
    let pool = db::pool(&app).await?;
    record_payment(&pool, customer_id, amount, notes, user_id).await
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
}
