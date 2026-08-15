use serde::{Deserialize, Serialize};
use sqlx::Row;
use tauri::{AppHandle, Runtime};

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
#[serde(rename_all = "camelCase")]
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
) -> Result<Vec<SupplierInvoiceRecord>, String> {
    let pool = db::pool(&app).await?;
    let rows = sqlx::query(
        "SELECT si.id, si.invoice_no, si.supplier_id, s.name, si.date,
                si.total, si.paid_amount, si.due_amount, si.status, si.notes
         FROM supplier_invoices si
         JOIN suppliers s ON s.id = si.supplier_id
         ORDER BY si.date DESC, si.id DESC",
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| e.to_string())?;

    rows.into_iter()
        .map(|row| {
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

    #[tokio::test]
    async fn create_invoice_records_items_stock_and_audit() {
        let path = std::env::temp_dir().join(format!(
            "store-pos-invoice-valid-test-{}.db",
            std::process::id()
        ));
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
            "SELECT COUNT(*), COALESCE(SUM(qty), 0) FROM stock_movements WHERE product_id = ? AND type = 'purchase_in' AND ref_id = ?",
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
        let path = std::env::temp_dir().join(format!(
            "store-pos-invoice-invalid-test-{}.db",
            std::process::id()
        ));
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
}
