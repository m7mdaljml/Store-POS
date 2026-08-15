use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Runtime};

use crate::db;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaleItemInput {
    pub product_id: i64,
    pub qty: f64,
    /// Unit sell price.
    pub price: f64,
    /// Snapshot of the product's cost price.
    pub cost_price: f64,
    /// Per-unit item discount.
    pub discount: f64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SalePaymentInput {
    pub method: String,
    pub amount: f64,
    pub reference: Option<String>,
    /// Required when method is "credit".
    pub customer_id: Option<i64>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSaleInput {
    pub items: Vec<SaleItemInput>,
    pub payments: Vec<SalePaymentInput>,
    /// Order-level discount amount (recomputed alongside item discounts).
    pub discount: f64,
    /// Tax amount for the order.
    pub tax: f64,
    pub user_id: Option<i64>,
    /// Optional customer attached to the sale (purchase history).
    pub customer_id: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaleResult {
    pub sale_id: i64,
    pub sale_no: String,
    pub subtotal: f64,
    pub discount: f64,
    pub tax: f64,
    pub total: f64,
    pub paid_amount: f64,
    pub change_given: f64,
}

fn optional_field(value: &Option<String>) -> Option<String> {
    value.as_ref().map(|v| v.trim()).filter(|v| !v.is_empty()).map(String::from)
}

/// Validates input and computes the authoritative order totals from item lines.
/// Returns `(subtotal, item_discount, order_discount, tax, total)`.
fn validate_and_totals(input: &CreateSaleInput) -> Result<(f64, f64, f64, f64, f64), String> {
    if input.items.is_empty() {
        return Err("Add at least one product to the cart".into());
    }
    if input.payments.is_empty() {
        return Err("Add at least one payment method".into());
    }
    for item in &input.items {
        if item.qty <= 0.0 {
            return Err("Quantities must be greater than zero".into());
        }
        if item.price < 0.0 {
            return Err("Sale price cannot be negative".into());
        }
        if item.discount < 0.0 || item.discount > item.price {
            return Err(format!("Item discount for product {} is out of range", item.product_id));
        }
    }

    let mut subtotal = 0.0;
    let mut item_discount = 0.0;
    for item in &input.items {
        subtotal += item.price * item.qty;
        item_discount += item.discount * item.qty;
    }
    let order_discount = input.discount.max(0.0);
    let tax = input.tax.max(0.0);
    let total = (subtotal - item_discount - order_discount + tax).max(0.0);
    Ok((subtotal, item_discount, order_discount, tax, total))
}

#[tauri::command]
pub async fn create_sale<R: Runtime>(
    app: AppHandle<R>,
    input: CreateSaleInput,
) -> Result<SaleResult, String> {
    let pool = db::pool(&app).await?;
    insert_sale(&pool, input).await
}

pub async fn insert_sale(
    pool: &sqlx::SqlitePool,
    input: CreateSaleInput,
) -> Result<SaleResult, String> {
    let (subtotal, item_discount, order_discount, tax, total) = validate_and_totals(&input)?;

    // Payments must cover the total (0.01 tolerance for float rounding).
    let payment_sum: f64 = input.payments.iter().map(|p| p.amount.max(0.0)).sum();
    if payment_sum + 0.005 < total {
        return Err(format!(
            "Payment amount is short by {}",
            (total - payment_sum).max(0.0)
        ));
    }

    for p in &input.payments {
        if !matches!(p.method.as_str(), "cash" | "card" | "credit") {
            return Err(format!("Unknown payment method '{}'", p.method));
        }
        if p.amount <= 0.0 {
            return Err("Payment amounts must be greater than zero".into());
        }
        if p.method == "credit" && p.customer_id.is_none() {
            return Err("Customer credit payments require a customer".into());
        }
    }

    // Change = cash tendered beyond the amount still due after non-cash payments.
    let non_cash: f64 = input
        .payments
        .iter()
        .filter(|p| p.method != "cash")
        .map(|p| p.amount)
        .sum();
    let cash_sum: f64 = input
        .payments
        .iter()
        .filter(|p| p.method == "cash")
        .map(|p| p.amount)
        .sum();
    let paid_amount = payment_sum.min(total);
    let change_given = (cash_sum - (total - non_cash).max(0.0)).max(0.0);

    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    for item in &input.items {
        let product: Option<(f64, String)> =
            sqlx::query_as("SELECT stock_qty, name FROM products WHERE id = ?")
                .bind(item.product_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
        let (stock, name) =
            product.ok_or_else(|| format!("Product {} not found", item.product_id))?;
        if stock + 0.005 < item.qty {
            return Err(format!(
                "Insufficient stock for \"{name}\": have {stock}, need {}",
                item.qty
            ));
        }
    }

    // Credit customers must exist; resolve the sale's customer.
    let mut sale_customer_id = input.customer_id;
    for p in &input.payments {
        if p.method != "credit" {
            continue;
        }
        let cid = p.customer_id.unwrap();
        let exists: Option<(i64,)> =
            sqlx::query_as("SELECT id FROM customers WHERE id = ?")
                .bind(cid)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
        if exists.is_none() {
            return Err(format!("Customer {cid} not found"));
        }
        sale_customer_id = Some(cid);
    }

    let next_no: (i64,) =
        sqlx::query_as("SELECT COALESCE(MAX(id), 0) + 1 FROM sales")
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
    let sale_no = format!("S-{:06}", next_no.0);

    let result = sqlx::query(
        "INSERT INTO sales
            (sale_no, customer_id, user_id, subtotal, discount, tax, total, paid_amount, change_given, status)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'completed')",
    )
    .bind(&sale_no)
    .bind(sale_customer_id)
    .bind(input.user_id)
    .bind(subtotal)
    .bind(order_discount)
    .bind(tax)
    .bind(total)
    .bind(paid_amount)
    .bind(change_given)
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    let sale_id = result.last_insert_rowid();

    for item in &input.items {
        let item_subtotal = (item.price - item.discount) * item.qty;

        sqlx::query(
            "INSERT INTO sale_items (sale_id, product_id, qty, price, cost_price, discount, tax, subtotal)
             VALUES (?, ?, ?, ?, ?, ?, 0, ?)",
        )
        .bind(sale_id)
        .bind(item.product_id)
        .bind(item.qty)
        .bind(item.price)
        .bind(item.cost_price)
        .bind(item.discount)
        .bind(item_subtotal)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

        sqlx::query(
            "INSERT INTO stock_movements (product_id, type, qty, ref_id, notes, user_id)
             VALUES (?, 'sale_out', ?, ?, ?, ?)",
        )
        .bind(item.product_id)
        .bind(-item.qty)
        .bind(sale_id)
        .bind(format!("Sale {sale_no}"))
        .bind(input.user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

        sqlx::query("UPDATE products SET stock_qty = stock_qty - ? WHERE id = ?")
            .bind(item.qty)
            .bind(item.product_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
    }

    for p in &input.payments {
        sqlx::query(
            "INSERT INTO payments (sale_id, method, amount, reference)
             VALUES (?, ?, ?, ?)",
        )
        .bind(sale_id)
        .bind(&p.method)
        .bind(p.amount)
        .bind(optional_field(&p.reference))
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

        if p.method == "credit" {
            let cid = p.customer_id.unwrap();
            let current: (f64,) = sqlx::query_as("SELECT balance FROM customers WHERE id = ?")
                .bind(cid)
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
            let balance_after = current.0 + p.amount;

            sqlx::query("UPDATE customers SET balance = ? WHERE id = ?")
                .bind(balance_after)
                .bind(cid)
                .execute(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;

            sqlx::query(
                "INSERT INTO customer_ledger (customer_id, sale_id, type, amount, balance_after, notes, user_id)
                 VALUES (?, ?, 'charge', ?, ?, ?, ?)",
            )
            .bind(cid)
            .bind(sale_id)
            .bind(p.amount)
            .bind(balance_after)
            .bind(format!("Sale {sale_no}"))
            .bind(input.user_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
        }
    }

    sqlx::query(
        "INSERT INTO audit_log (user_id, action, entity_type, entity_id, details)
         VALUES (?, 'sale.create', 'sale', ?, ?)",
    )
    .bind(input.user_id)
    .bind(sale_id)
    .bind(format!(
        "Created sale {sale_no} for {} item(s), total {}, paid {}, change {}",
        input.items.len(),
        total,
        paid_amount,
        change_given
    ))
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    tx.commit().await.map_err(|e| e.to_string())?;

    Ok(SaleResult {
        sale_id,
        sale_no,
        subtotal,
        discount: item_discount + order_discount,
        tax,
        total,
        paid_amount,
        change_given,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schema() -> &'static str {
        r#"
        CREATE TABLE users (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          username TEXT NOT NULL
        );
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
        CREATE TABLE products (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          name TEXT NOT NULL,
          category_id INTEGER,
          barcode TEXT,
          sku TEXT,
          stock_qty REAL NOT NULL DEFAULT 0,
          unit_price REAL NOT NULL DEFAULT 0,
          cost_price REAL NOT NULL DEFAULT 0
        );
        CREATE TABLE sales (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          sale_no TEXT UNIQUE NOT NULL,
          session_id INTEGER,
          customer_id INTEGER,
          user_id INTEGER NOT NULL,
          currency_id INTEGER,
          subtotal REAL NOT NULL DEFAULT 0,
          discount REAL NOT NULL DEFAULT 0,
          tax REAL NOT NULL DEFAULT 0,
          total REAL NOT NULL DEFAULT 0,
          paid_amount REAL NOT NULL DEFAULT 0,
          change_given REAL NOT NULL DEFAULT 0,
          status TEXT NOT NULL DEFAULT 'completed',
          void_reason TEXT,
          voided_by INTEGER,
          created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE TABLE sale_items (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          sale_id INTEGER NOT NULL REFERENCES sales(id) ON DELETE CASCADE,
          product_id INTEGER NOT NULL,
          qty REAL NOT NULL,
          price REAL NOT NULL,
          cost_price REAL NOT NULL,
          discount REAL NOT NULL DEFAULT 0,
          tax REAL NOT NULL DEFAULT 0,
          subtotal REAL NOT NULL
        );
        CREATE TABLE payments (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          sale_id INTEGER NOT NULL REFERENCES sales(id) ON DELETE CASCADE,
          method TEXT NOT NULL,
          amount REAL NOT NULL,
          reference TEXT,
          created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE TABLE stock_movements (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          product_id INTEGER NOT NULL,
          type TEXT NOT NULL,
          qty REAL NOT NULL,
          ref_id INTEGER,
          notes TEXT,
          user_id INTEGER,
          created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE TABLE customer_ledger (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          customer_id INTEGER NOT NULL,
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

    async fn sample_product(pool: &sqlx::SqlitePool, id: i64, name: &str, stock: f64, price: f64) {
        sqlx::query(
            "INSERT INTO products (id, name, stock_qty, unit_price, cost_price) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(name)
        .bind(stock)
        .bind(price)
        .bind(price * 0.6)
        .execute(pool)
        .await
        .unwrap();
    }

    async fn sample_customer(pool: &sqlx::SqlitePool, id: i64, name: &str) {
        sqlx::query("INSERT INTO customers (id, name) VALUES (?, ?)")
            .bind(id)
            .bind(name)
            .execute(pool)
            .await
            .unwrap();
    }

    async fn mem_pool() -> sqlx::SqlitePool {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::raw_sql(schema()).execute(&pool).await.unwrap();
        pool
    }

    fn item(product_id: i64, qty: f64, price: f64, discount: f64) -> SaleItemInput {
        SaleItemInput {
            product_id,
            qty,
            price,
            cost_price: price * 0.6,
            discount,
        }
    }

    #[tokio::test]
    async fn create_sale_records_everything() {
        let pool = mem_pool().await;
        sample_product(&pool, 1, "Coffee", 20.0, 5.0).await;
        sample_product(&pool, 2, "Tea", 10.0, 3.0).await;

        let input = CreateSaleInput {
            items: vec![item(1, 2.0, 5.0, 0.0), item(2, 1.0, 3.0, 0.5)],
            payments: vec![SalePaymentInput {
                method: "cash".into(),
                amount: 15.0,
                reference: None,
                customer_id: None,
            }],
            discount: 1.0,
            tax: 0.0,
            user_id: Some(1),
            customer_id: None,
        };
        let res = insert_sale(&pool, input).await.unwrap();
        assert_eq!(res.sale_no, "S-000001");
        // subtotal = 2*5 + 1*3 = 13; item discount = 0.5; order discount = 1
        // total = 13 - 0.5 - 1 = 11.5; paid 15 -> change 3.5
        assert!((res.subtotal - 13.0).abs() < 0.001);
        assert!((res.total - 11.5).abs() < 0.001);
        assert!((res.paid_amount - 11.5).abs() < 0.001);
        assert!((res.change_given - 3.5).abs() < 0.001);
        assert_eq!(res.discount, 1.5);

        let sale: (String, i64, f64, f64) =
            sqlx::query_as("SELECT sale_no, user_id, total, change_given FROM sales WHERE id = ?")
                .bind(res.sale_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(sale.0, "S-000001");
        assert_eq!(sale.1, 1);
        assert!((sale.2 - 11.5).abs() < 0.001);
        assert!((sale.3 - 3.5).abs() < 0.001);

        let items: (i64, i64) =
            sqlx::query_as("SELECT COUNT(*), COUNT(DISTINCT product_id) FROM sale_items WHERE sale_id = ?")
                .bind(res.sale_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(items, (2, 2));

        let payments: (i64, String, f64) =
            sqlx::query_as("SELECT COUNT(*), method, amount FROM payments WHERE sale_id = ?")
                .bind(res.sale_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(payments.0, 1);
        assert_eq!(payments.1, "cash");
        assert!((payments.2 - 15.0).abs() < 0.001);

        let moves: Vec<(String, f64)> = sqlx::query_as(
            "SELECT type, qty FROM stock_movements WHERE product_id = ? AND ref_id = ?",
        )
        .bind(1)
        .bind(res.sale_id)
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(moves.len(), 1);
        assert_eq!(moves[0].0, "sale_out");
        assert!((moves[0].1 + 2.0).abs() < 0.001);

        let stock: (f64,) =
            sqlx::query_as("SELECT stock_qty FROM products WHERE id = 1")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!((stock.0 - 18.0).abs() < 0.001);

        let audit: (String, String, i64, String) = sqlx::query_as(
            "SELECT action, entity_type, entity_id, details FROM audit_log WHERE entity_id = ?",
        )
        .bind(res.sale_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(audit.0, "sale.create");
        assert_eq!(audit.1, "sale");
        assert_eq!(audit.2, res.sale_id);
        assert!(audit.3.contains("S-000001"));
    }

    #[tokio::test]
    async fn create_sale_with_credit_updates_ledger_and_balance() {
        let pool = mem_pool().await;
        sample_product(&pool, 1, "Coffee", 20.0, 5.0).await;
        sample_customer(&pool, 1, "Alice").await;

        let input = CreateSaleInput {
            items: vec![item(1, 2.0, 5.0, 0.0)],
            payments: vec![
                SalePaymentInput {
                    method: "credit".into(),
                    amount: 6.0,
                    reference: None,
                    customer_id: Some(1),
                },
                SalePaymentInput {
                    method: "cash".into(),
                    amount: 4.0,
                    reference: None,
                    customer_id: None,
                },
            ],
            discount: 0.0,
            tax: 0.0,
            user_id: Some(1),
            customer_id: None,
        };
        let res = insert_sale(&pool, input).await.unwrap();
        assert!((res.total - 10.0).abs() < 0.001);

        let balance: (f64,) = sqlx::query_as("SELECT balance FROM customers WHERE id = 1")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!((balance.0 - 6.0).abs() < 0.001);

        let ledger: (String, f64, f64) = sqlx::query_as(
            "SELECT type, amount, balance_after FROM customer_ledger WHERE customer_id = 1",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(ledger.0, "charge");
        assert!((ledger.1 - 6.0).abs() < 0.001);
        assert!((ledger.2 - 6.0).abs() < 0.001);

        // Sale customer should point at the credit customer.
        let cust: (i64,) = sqlx::query_as("SELECT customer_id FROM sales WHERE id = ?")
            .bind(res.sale_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(cust.0, 1);

        // Split payment: two rows recorded.
        let n: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM payments WHERE sale_id = ?")
            .bind(res.sale_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(n.0, 2);
    }

    #[tokio::test]
    async fn create_sale_rejects_bad_input() {
        let pool = mem_pool().await;
        sample_product(&pool, 1, "Coffee", 2.0, 5.0).await;
        sample_customer(&pool, 1, "Alice").await;

        let base = || CreateSaleInput {
            items: vec![item(1, 1.0, 5.0, 0.0)],
            payments: vec![SalePaymentInput {
                method: "cash".into(),
                amount: 5.0,
                reference: None,
                customer_id: None,
            }],
            discount: 0.0,
            tax: 0.0,
            user_id: Some(1),
            customer_id: None,
        };

        // Empty cart.
        let mut bad = base();
        bad.items.clear();
        assert!(insert_sale(&pool, bad).await.is_err());

        // No payments.
        let mut bad = base();
        bad.payments.clear();
        assert!(insert_sale(&pool, bad).await.is_err());

        // Underpaid.
        let mut bad = base();
        bad.payments[0].amount = 3.0;
        let err = insert_sale(&pool, bad).await.unwrap_err();
        assert!(err.contains("short"));

        // Unknown method.
        let mut bad = base();
        bad.payments[0].method = "bitcoin".into();
        assert!(insert_sale(&pool, bad).await.is_err());

        // Credit without a customer.
        let mut bad = base();
        bad.payments[0].method = "credit".into();
        assert!(insert_sale(&pool, bad).await.is_err());

        // Insufficient stock.
        let mut bad = base();
        bad.items[0].qty = 3.0;
        bad.payments[0].amount = 15.0;
        let err = insert_sale(&pool, bad).await.unwrap_err();
        assert!(err.contains("Insufficient stock"));

        // Failed validation must not leave partial rows behind.
        let n: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM sales")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(n.0, 0);
    }

    #[tokio::test]
    async fn sale_no_increments_per_sale() {
        let pool = mem_pool().await;
        sample_product(&pool, 1, "Coffee", 100.0, 5.0).await;

        let input = || CreateSaleInput {
            items: vec![item(1, 1.0, 5.0, 0.0)],
            payments: vec![SalePaymentInput {
                method: "cash".into(),
                amount: 5.0,
                reference: None,
                customer_id: None,
            }],
            discount: 0.0,
            tax: 0.0,
            user_id: Some(1),
            customer_id: None,
        };

        let first = insert_sale(&pool, input()).await.unwrap();
        let second = insert_sale(&pool, input()).await.unwrap();
        assert_eq!(first.sale_no, "S-000001");
        assert_eq!(second.sale_no, "S-000002");
        assert_ne!(first.sale_id, second.sale_id);
    }
}
