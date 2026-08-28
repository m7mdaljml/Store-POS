use serde::{Deserialize, Serialize};
use sqlx::Row;
use tauri::{AppHandle, Runtime};

use super::Page;
use crate::db;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceItemInput {
    pub product_id: i64,
    pub qty: f64,
    pub cost_price: f64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SupplierInvoiceInput {
    pub supplier_id: i64,
    pub date: Option<String>,
    pub notes: Option<String>,
    pub items: Vec<InvoiceItemInput>,
    pub user_id: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct SupplierInvoiceRecord {
    pub id: i64,
    pub invoice_no: String,
    pub supplier_id: i64,
    pub supplier_name: String,
    pub date: String,
    pub total: f64,
    pub paid_amount: f64,
    pub due_amount: f64,
    pub status: String,
    pub notes: Option<String>,
}

fn optional_field(value: &Option<String>) -> Option<String> {
    value.as_ref().map(|v| v.trim()).filter(|v| !v.is_empty()).map(String::from)
}

#[tauri::command]
pub async fn create_supplier_invoice<R: Runtime>(
    app: AppHandle<R>,
    input: SupplierInvoiceInput,
) -> Result<i64, String> {
    let pool = db::pool(&app).await?;
    create_invoice(&pool, input).await
}

pub async fn create_invoice(
    pool: &sqlx::SqlitePool,
    input: SupplierInvoiceInput,
) -> Result<i64, String> {
    if input.items.is_empty() {
        return Err("Add at least one product line".into());
    }
    for item in &input.items {
        if item.qty <= 0.0 {
            return Err("Quantities must be greater than zero".into());
        }
        if item.cost_price < 0.0 {
            return Err("Cost price cannot be negative".into());
        }
    }

    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    let supplier: Option<(i64,)> =
        sqlx::query_as("SELECT id FROM suppliers WHERE id = ?")
            .bind(input.supplier_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
    if supplier.is_none() {
        return Err("Supplier not found".into());
    }

    for item in &input.items {
        let product: Option<(i64,)> =
            sqlx::query_as("SELECT id FROM products WHERE id = ?")
                .bind(item.product_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
        if product.is_none() {
            return Err(format!("Product {} not found", item.product_id));
        }
    }

    let total: f64 = input.items.iter().map(|i| i.qty * i.cost_price).sum();
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

    let next_no: (i64,) =
        sqlx::query_as("SELECT COALESCE(MAX(id), 0) + 1 FROM supplier_invoices")
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
    let invoice_no = format!("PI-{:06}", next_no.0);

    let result = sqlx::query(
        "INSERT INTO supplier_invoices
            (invoice_no, supplier_id, date, total, paid_amount, due_amount, status, notes, user_id)
         VALUES (?, ?, ?, ?, 0, ?, 'unpaid', ?, ?)",
    )
    .bind(&invoice_no)
    .bind(input.supplier_id)
    .bind(&date)
    .bind(total)
    .bind(total)
    .bind(optional_field(&input.notes))
    .bind(input.user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    let invoice_id = result.last_insert_rowid();

    for item in &input.items {
        let subtotal = item.qty * item.cost_price;
        sqlx::query(
            "INSERT INTO supplier_invoice_items (invoice_id, product_id, qty, cost_price, subtotal)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(invoice_id)
        .bind(item.product_id)
        .bind(item.qty)
        .bind(item.cost_price)
        .bind(subtotal)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

        sqlx::query(
            "INSERT INTO stock_movements (product_id, type, qty, ref_id, notes, user_id)
             VALUES (?, 'purchase_in', ?, ?, ?, ?)",
        )
        .bind(item.product_id)
        .bind(item.qty)
        .bind(invoice_id)
        .bind(format!("Supplier invoice {invoice_no}"))
        .bind(input.user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

        sqlx::query("UPDATE products SET stock_qty = stock_qty + ? WHERE id = ?")
            .bind(item.qty)
            .bind(item.product_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
    }

    sqlx::query(
        "INSERT INTO audit_log (user_id, action, entity_type, entity_id, details)
         VALUES (?, 'expense.incoming', 'supplier_invoice', ?, ?)",
    )
    .bind(input.user_id)
    .bind(invoice_id)
    .bind(format!("Created incoming invoice {invoice_no} for {} line(s)", input.items.len()))
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(invoice_id)
}

#[tauri::command]
pub async fn list_supplier_invoices<R: Runtime>(
    app: AppHandle<R>,
    search: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Page<SupplierInvoiceRecord>, String> {
    let pool = db::pool(&app).await?;

    let pattern = search
        .as_deref()
        .map(|s| format!("%{}%", s.trim()))
        .filter(|p| p != "%%");
    let search_cond = if pattern.is_some() {
        " WHERE (si.invoice_no LIKE ? OR s.name LIKE ?)"
    } else {
        ""
    };
    let sql = format!(
        "SELECT si.id, si.invoice_no, si.supplier_id, s.name AS supplier_name, si.date,
                si.total, si.paid_amount, si.due_amount, si.status, si.notes
         FROM supplier_invoices si
         JOIN suppliers s ON s.id = si.supplier_id
         {search_cond}
         ORDER BY si.date DESC, si.id DESC
         LIMIT ? OFFSET ?"
    );

    let mut query = sqlx::query(&sql);
    if let Some(p) = &pattern {
        query = query.bind(p).bind(p);
    }
    let rows = query
        .bind(limit.map(|l| l.max(1)).unwrap_or(-1))
        .bind(offset.unwrap_or(0).max(0))
        .fetch_all(&pool)
        .await
        .map_err(|e| e.to_string())?;

    let items: Vec<SupplierInvoiceRecord> = rows
        .into_iter()
        .map(|row| -> Result<SupplierInvoiceRecord, String> {
            Ok(SupplierInvoiceRecord {
                id: row.try_get("id").map_err(|e| e.to_string())?,
                invoice_no: row.try_get("invoice_no").map_err(|e| e.to_string())?,
                supplier_id: row.try_get("supplier_id").map_err(|e| e.to_string())?,
                supplier_name: row.try_get("supplier_name").map_err(|e| e.to_string())?,
                date: row.try_get("date").map_err(|e| e.to_string())?,
                total: row.try_get("total").map_err(|e| e.to_string())?,
                paid_amount: row.try_get("paid_amount").map_err(|e| e.to_string())?,
                due_amount: row.try_get("due_amount").map_err(|e| e.to_string())?,
                status: row.try_get("status").map_err(|e| e.to_string())?,
                notes: row.try_get("notes").map_err(|e| e.to_string())?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    let count_sql = format!(
        "SELECT COUNT(*) FROM supplier_invoices si
         JOIN suppliers s ON s.id = si.supplier_id
         {search_cond}"
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

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SupplierPaymentInput {
    pub invoice_id: i64,
    pub amount: f64,
    pub method: Option<String>,
    pub date: Option<String>,
    pub notes: Option<String>,
    pub user_id: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct SupplierPaymentRecord {
    pub id: i64,
    pub invoice_id: i64,
    pub invoice_no: String,
    pub amount: f64,
    pub method: String,
    pub date: String,
    pub notes: Option<String>,
    pub user_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PaymentResult {
    pub paid_amount: f64,
    pub due_amount: f64,
    pub status: String,
}

pub async fn record_supplier_payment(
    pool: &sqlx::SqlitePool,
    input: SupplierPaymentInput,
) -> Result<PaymentResult, String> {
    if input.amount <= 0.0 {
        return Err("Payment amount must be greater than zero".into());
    }
    let method = optional_field(&input.method).unwrap_or_else(|| "cash".into());
    if !matches!(method.as_str(), "cash" | "card" | "bank") {
        return Err("Payment method must be cash, card or bank".into());
    }

    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    let invoice: Option<(String, f64, f64, f64)> = sqlx::query_as(
        "SELECT invoice_no, total, paid_amount, due_amount FROM supplier_invoices WHERE id = ?",
    )
    .bind(input.invoice_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    let (invoice_no, total, paid, due) = match invoice {
        Some(v) => v,
        None => return Err("Invoice not found".into()),
    };

    if input.amount > due + 0.005 {
        return Err(format!(
            "Payment exceeds the outstanding amount ({due:.2} remaining)"
        ));
    }

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

    sqlx::query(
        "INSERT INTO supplier_payments (invoice_id, amount, method, date, notes, user_id)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(input.invoice_id)
    .bind(input.amount)
    .bind(&method)
    .bind(&date)
    .bind(optional_field(&input.notes))
    .bind(input.user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    let new_paid = paid + input.amount;
    let new_due = (total - new_paid).max(0.0);
    let status = if new_due <= 0.005 { "paid" } else { "partial" };

    sqlx::query(
        "UPDATE supplier_invoices SET paid_amount = ?, due_amount = ?, status = ? WHERE id = ?",
    )
    .bind(new_paid)
    .bind(new_due)
    .bind(status)
    .bind(input.invoice_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query(
        "INSERT INTO audit_log (user_id, action, entity_type, entity_id, details)
         VALUES (?, 'expense.payment', 'supplier_invoice', ?, ?)",
    )
    .bind(input.user_id)
    .bind(input.invoice_id)
    .bind(format!(
        "Payment of {} ({}) on invoice {}",
        crate::format::money(input.amount),
        method,
        invoice_no
    ))
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    tx.commit().await.map_err(|e| e.to_string())?;

    Ok(PaymentResult {
        paid_amount: new_paid,
        due_amount: new_due,
        status: status.into(),
    })
}

#[tauri::command]
pub async fn add_supplier_payment<R: Runtime>(
    app: AppHandle<R>,
    input: SupplierPaymentInput,
) -> Result<PaymentResult, String> {
    let pool = db::pool(&app).await?;
    record_supplier_payment(&pool, input).await
}

#[tauri::command]
pub async fn list_supplier_payments<R: Runtime>(
    app: AppHandle<R>,
    invoice_id: i64,
) -> Result<Vec<SupplierPaymentRecord>, String> {
    let pool = db::pool(&app).await?;
    let rows = sqlx::query(
        "SELECT sp.id, sp.invoice_id, si.invoice_no, sp.amount, sp.method, sp.date, sp.notes, u.full_name AS user_name
         FROM supplier_payments sp
         JOIN supplier_invoices si ON si.id = sp.invoice_id
         LEFT JOIN users u ON u.id = sp.user_id
         WHERE sp.invoice_id = ?
         ORDER BY sp.date DESC, sp.id DESC",
    )
    .bind(invoice_id)
    .fetch_all(&pool)
    .await
    .map_err(|e| e.to_string())?;

    rows.into_iter()
        .map(|row| {
            Ok(SupplierPaymentRecord {
                id: row.try_get("id").map_err(|e| e.to_string())?,
                invoice_id: row.try_get("invoice_id").map_err(|e| e.to_string())?,
                invoice_no: row.try_get("invoice_no").map_err(|e| e.to_string())?,
                amount: row.try_get("amount").map_err(|e| e.to_string())?,
                method: row.try_get("method").map_err(|e| e.to_string())?,
                date: row.try_get("date").map_err(|e| e.to_string())?,
                notes: row.try_get("notes").map_err(|e| e.to_string())?,
                user_name: row.try_get("user_name").map_err(|e| e.to_string())?,
            })
        })
        .collect()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::db;

    pub(crate) async fn create_schema(path: &std::path::Path) {
        let pool = db::connect(path).await.unwrap();
        for sql in [
            "CREATE TABLE suppliers (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, contact TEXT, phone TEXT, email TEXT, address TEXT, tax_id TEXT, created_at TEXT NOT NULL DEFAULT (datetime('now')))",
            "CREATE TABLE products (id INTEGER PRIMARY KEY AUTOINCREMENT, sku TEXT, barcode TEXT, name TEXT NOT NULL, description TEXT, category_id INTEGER, cost_price REAL NOT NULL DEFAULT 0, sell_price REAL NOT NULL DEFAULT 0, tax_profile_id INTEGER, unit TEXT NOT NULL DEFAULT 'pc', stock_qty REAL NOT NULL DEFAULT 0, reorder_level REAL NOT NULL DEFAULT 0, image_path TEXT, is_active INTEGER NOT NULL DEFAULT 1, created_at TEXT NOT NULL DEFAULT (datetime('now')), updated_at TEXT NOT NULL DEFAULT (datetime('now')))",
            "CREATE TABLE supplier_invoices (id INTEGER PRIMARY KEY AUTOINCREMENT, invoice_no TEXT NOT NULL, supplier_id INTEGER NOT NULL, date TEXT NOT NULL, total REAL NOT NULL DEFAULT 0, paid_amount REAL NOT NULL DEFAULT 0, due_amount REAL NOT NULL DEFAULT 0, status TEXT NOT NULL DEFAULT 'unpaid', notes TEXT, user_id INTEGER, created_at TEXT NOT NULL DEFAULT (datetime('now')))",
            "CREATE TABLE supplier_invoice_items (id INTEGER PRIMARY KEY AUTOINCREMENT, invoice_id INTEGER NOT NULL, product_id INTEGER NOT NULL, qty REAL NOT NULL, cost_price REAL NOT NULL, subtotal REAL NOT NULL)",
            "CREATE TABLE stock_movements (id INTEGER PRIMARY KEY AUTOINCREMENT, product_id INTEGER NOT NULL, type TEXT NOT NULL, qty REAL NOT NULL, ref_id INTEGER, notes TEXT, user_id INTEGER, created_at TEXT NOT NULL DEFAULT (datetime('now')))",
            "CREATE TABLE supplier_payments (id INTEGER PRIMARY KEY AUTOINCREMENT, invoice_id INTEGER NOT NULL, amount REAL NOT NULL, method TEXT NOT NULL DEFAULT 'cash', date TEXT NOT NULL DEFAULT (datetime('now')), notes TEXT, user_id INTEGER)",
            "CREATE TABLE audit_log (id INTEGER PRIMARY KEY AUTOINCREMENT, user_id INTEGER, action TEXT NOT NULL, entity_type TEXT, entity_id INTEGER, details TEXT, created_at TEXT NOT NULL DEFAULT (datetime('now')))",
        ] {
            sqlx::query(sql).execute(&pool).await.unwrap();
        }
        pool.close().await;
    }

    fn sample_input(supplier_id: i64, items: Vec<InvoiceItemInput>) -> SupplierInvoiceInput {
        SupplierInvoiceInput {
            supplier_id,
            date: Some("2026-08-15".into()),
            notes: Some("Test delivery".into()),
            items,
            user_id: None,
        }
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
    async fn create_invoice_records_items_stock_and_audit() {
        let path = temp_db("invoice-valid");
        create_schema(&path).await;
        let pool = db::connect(&path).await.unwrap();

        let supplier_id = sqlx::query("INSERT INTO suppliers (name) VALUES ('Test Supplier')")
            .execute(&pool)
            .await
            .unwrap()
            .last_insert_rowid();
        let product_id = sqlx::query(
            "INSERT INTO products (name, unit, stock_qty) VALUES ('Widget', 'pc', 10)",
        )
        .execute(&pool)
        .await
        .unwrap()
        .last_insert_rowid();

        let invoice_id = create_invoice(
            &pool,
            sample_input(
                supplier_id,
                vec![
                    InvoiceItemInput { product_id, qty: 3.0, cost_price: 4.0 },
                    InvoiceItemInput { product_id, qty: 2.0, cost_price: 5.0 },
                ],
            ),
        )
        .await
        .unwrap();

        let (no, total, paid, due, status): (String, f64, f64, f64, String) =
            sqlx::query_as("SELECT invoice_no, total, paid_amount, due_amount, status FROM supplier_invoices WHERE id = ?")
                .bind(invoice_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(total, 22.0, "total = 3*4 + 2*5");
        assert_eq!(paid, 0.0);
        assert_eq!(due, 22.0);
        assert_eq!(status, "unpaid");
        assert_eq!(no, "PI-000001");

        let item_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM supplier_invoice_items WHERE invoice_id = ?",
        )
        .bind(invoice_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(item_count, 2);

        let movement_sum: (i64, f64) = sqlx::query_as(
            "SELECT COUNT(*), COALESCE(SUM(qty), 0.0) FROM stock_movements WHERE product_id = ? AND type = 'purchase_in' AND ref_id = ?",
        )
        .bind(product_id)
        .bind(invoice_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(movement_sum, (2, 5.0));

        let stock: f64 = sqlx::query_scalar("SELECT stock_qty FROM products WHERE id = ?")
            .bind(product_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(stock, 15.0, "10 + 3 + 2");

        let audit: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM audit_log WHERE action = 'expense.incoming' AND entity_id = ?",
        )
        .bind(invoice_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(audit, 1);

        pool.close().await;
        std::fs::remove_file(&path).ok();
    }

    #[tokio::test]
    async fn create_invoice_rejects_invalid_input() {
        let path = temp_db("invoice-invalid");
        create_schema(&path).await;
        let pool = db::connect(&path).await.unwrap();

        let supplier_id = sqlx::query("INSERT INTO suppliers (name) VALUES ('Test Supplier')")
            .execute(&pool)
            .await
            .unwrap()
            .last_insert_rowid();
        let product_id = sqlx::query(
            "INSERT INTO products (name, unit, stock_qty) VALUES ('Widget', 'pc', 10)",
        )
        .execute(&pool)
        .await
        .unwrap()
        .last_insert_rowid();

        let bad_qty = create_invoice(
            &pool,
            sample_input(
                supplier_id,
                vec![InvoiceItemInput { product_id, qty: 0.0, cost_price: 4.0 }],
            ),
        )
        .await;
        assert!(bad_qty.is_err());

        let missing_product = create_invoice(
            &pool,
            sample_input(
                supplier_id,
                vec![InvoiceItemInput { product_id: 9999, qty: 1.0, cost_price: 4.0 }],
            ),
        )
        .await;
        assert!(missing_product.is_err());

        let empty_items = create_invoice(&pool, sample_input(supplier_id, vec![])).await;
        assert!(empty_items.is_err());

        let invoices: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM supplier_invoices")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(invoices, 0, "no partial invoice rows after failures");
        let stock: f64 = sqlx::query_scalar("SELECT stock_qty FROM products WHERE id = ?")
            .bind(product_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(stock, 10.0, "stock unchanged after failures");

        pool.close().await;
        std::fs::remove_file(&path).ok();
    }

    #[tokio::test]
    async fn payment_updates_paid_due_and_status() {
        let path = temp_db("payment-valid");
        create_schema(&path).await;
        let pool = db::connect(&path).await.unwrap();

        let invoice_id = sqlx::query(
            "INSERT INTO supplier_invoices (invoice_no, supplier_id, date, total, due_amount) VALUES ('PI-000001', 1, '2026-08-15', 100, 100)",
        )
        .execute(&pool)
        .await
        .unwrap()
        .last_insert_rowid();

        let first = record_supplier_payment(
            &pool,
            SupplierPaymentInput {
                invoice_id,
                amount: 40.0,
                method: Some("cash".into()),
                date: Some("2026-08-15".into()),
                notes: None,
                user_id: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(first.paid_amount, 40.0);
        assert_eq!(first.due_amount, 60.0);
        assert_eq!(first.status, "partial");

        let second = record_supplier_payment(
            &pool,
            SupplierPaymentInput {
                invoice_id,
                amount: 60.0,
                method: Some("bank".into()),
                date: Some("2026-08-16".into()),
                notes: None,
                user_id: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(second.paid_amount, 100.0);
        assert_eq!(second.due_amount, 0.0);
        assert_eq!(second.status, "paid");

        let (paid, due, status): (f64, f64, String) = sqlx::query_as(
            "SELECT paid_amount, due_amount, status FROM supplier_invoices WHERE id = ?",
        )
        .bind(invoice_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(paid, 100.0);
        assert_eq!(due, 0.0);
        assert_eq!(status, "paid");

        let payments: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM supplier_payments WHERE invoice_id = ?",
        )
        .bind(invoice_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(payments, 2);

        let audit: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM audit_log WHERE action = 'expense.payment' AND entity_id = ?",
        )
        .bind(invoice_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(audit, 2);

        pool.close().await;
        std::fs::remove_file(&path).ok();
    }

    #[tokio::test]
    async fn payment_rejects_overpayment_and_invalid_input() {
        let path = temp_db("payment-invalid");
        create_schema(&path).await;
        let pool = db::connect(&path).await.unwrap();

        let invoice_id = sqlx::query(
            "INSERT INTO supplier_invoices (invoice_no, supplier_id, date, total, due_amount) VALUES ('PI-000001', 1, '2026-08-15', 100, 100)",
        )
        .execute(&pool)
        .await
        .unwrap()
        .last_insert_rowid();

        let over = record_supplier_payment(
            &pool,
            SupplierPaymentInput {
                invoice_id,
                amount: 101.0,
                method: Some("cash".into()),
                date: None,
                notes: None,
                user_id: None,
            },
        )
        .await;
        assert!(over.is_err());

        let zero = record_supplier_payment(
            &pool,
            SupplierPaymentInput {
                invoice_id,
                amount: 0.0,
                method: Some("cash".into()),
                date: None,
                notes: None,
                user_id: None,
            },
        )
        .await;
        assert!(zero.is_err());

        let bad_method = record_supplier_payment(
            &pool,
            SupplierPaymentInput {
                invoice_id,
                amount: 10.0,
                method: Some("bitcoin".into()),
                date: None,
                notes: None,
                user_id: None,
            },
        )
        .await;
        assert!(bad_method.is_err());

        let missing = record_supplier_payment(
            &pool,
            SupplierPaymentInput {
                invoice_id: 9999,
                amount: 10.0,
                method: None,
                date: None,
                notes: None,
                user_id: None,
            },
        )
        .await;
        assert!(missing.is_err());

        let (paid, due, status): (f64, f64, String) = sqlx::query_as(
            "SELECT paid_amount, due_amount, status FROM supplier_invoices WHERE id = ?",
        )
        .bind(invoice_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(paid, 0.0);
        assert_eq!(due, 100.0);
        assert_eq!(status, "unpaid", "unchanged after all failed payments");

        pool.close().await;
        std::fs::remove_file(&path).ok();
    }
}
