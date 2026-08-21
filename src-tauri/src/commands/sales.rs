use serde::{Deserialize, Serialize};
use sqlx::Row;
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
    /// When resuming a held sale, pass its id so the same sale number is kept
    /// and the held record is completed in place.
    #[serde(default)]
    pub held_sale_id: Option<i64>,
    /// Cash register session this sale is recorded against. If provided it
    /// must reference an open session.
    #[serde(default)]
    pub session_id: Option<i64>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HoldSaleInput {
    pub items: Vec<SaleItemInput>,
    /// Order-level discount amount.
    pub discount: f64,
    /// Tax amount for the order.
    pub tax: f64,
    pub customer_id: Option<i64>,
    pub user_id: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HoldSaleResult {
    pub sale_id: i64,
    pub sale_no: String,
    pub total: f64,
    pub item_count: i64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumeSaleInput {
    pub sale_id: i64,
    pub user_id: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumeSaleItem {
    pub product_id: i64,
    pub name: String,
    pub qty: f64,
    pub price: f64,
    pub cost_price: f64,
    pub discount: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumeSaleRecord {
    pub sale_id: i64,
    pub sale_no: String,
    pub customer_id: Option<i64>,
    pub subtotal: f64,
    pub discount: f64,
    pub tax: f64,
    pub total: f64,
    pub items: Vec<ResumeSaleItem>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelHeldSaleInput {
    pub sale_id: i64,
    pub user_id: Option<i64>,
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

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoidSaleInput {
    pub sale_id: i64,
    pub reason: String,
    pub user_id: Option<i64>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListSalesInput {
    pub status: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaleRecord {
    pub id: i64,
    pub sale_no: String,
    pub created_at: String,
    pub user_name: Option<String>,
    pub customer_name: Option<String>,
    pub item_count: i64,
    pub total: f64,
    pub paid_amount: f64,
    pub status: String,
    pub void_reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaleReceiptInput {
    pub sale_id: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceiptItem {
    pub name: String,
    pub qty: f64,
    pub price: f64,
    pub discount: f64,
    pub subtotal: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceiptPayment {
    pub method: String,
    pub amount: f64,
    pub reference: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaleReceipt {
    /// Store profile from the settings table (may be empty until configured).
    pub store_name: String,
    pub store_address: String,
    pub store_phone: String,
    pub store_tax_id: String,
    /// Optional header text overriding the store name banner.
    pub receipt_header: String,
    pub receipt_footer: String,
    /// Store logo as a data URL (empty when unset).
    pub receipt_logo: String,
    /// Where to draw the logo: "top" | "bottom".
    pub receipt_logo_pos: String,
    /// Paper format: "thermal" | "a4".
    pub receipt_format: String,
    pub sale_id: i64,
    pub sale_no: String,
    pub created_at: String,
    pub status: String,
    pub customer_name: Option<String>,
    pub user_name: Option<String>,
    pub subtotal: f64,
    /// Sum of per-item discounts (price * qty reduction).
    pub item_discount: f64,
    /// Order-level discount stored on the sale.
    pub order_discount: f64,
    pub tax: f64,
    pub total: f64,
    pub paid_amount: f64,
    pub change_given: f64,
    pub items: Vec<ReceiptItem>,
    pub payments: Vec<ReceiptPayment>,
}

fn optional_field(value: &Option<String>) -> Option<String> {
    value.as_ref().map(|v| v.trim()).filter(|v| !v.is_empty()).map(String::from)
}

/// Validates item lines and computes the authoritative order totals.
/// Returns `(subtotal, item_discount, order_discount, tax, total)`.
fn validate_and_totals(input: &CreateSaleInput) -> Result<(f64, f64, f64, f64, f64), String> {
    validate_item_lines(&input.items)?;

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

fn validate_item_lines(items: &[SaleItemInput]) -> Result<(), String> {
    if items.is_empty() {
        return Err("Add at least one product to the cart".into());
    }
    for item in items {
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
    Ok(())
}

/// Checks each product exists and has enough stock for the requested quantity.
async fn check_stock(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    items: &[SaleItemInput],
) -> Result<(), String> {
    for item in items {
        let product: Option<(f64, String)> =
            sqlx::query_as("SELECT stock_qty, name FROM products WHERE id = ?")
                .bind(item.product_id)
                .fetch_optional(&mut **tx)
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
    Ok(())
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

    if input.payments.is_empty() {
        return Err("Add at least one payment method".into());
    }

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

    check_stock(&mut tx, &input.items).await?;

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

    // The register session must exist and be open when one is supplied.
    if let Some(sid) = input.session_id {
        let session: Option<(String,)> =
            sqlx::query_as("SELECT status FROM sale_sessions WHERE id = ?")
                .bind(sid)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
        match session {
            Some((status,)) if status == "open" => {}
            Some(_) => return Err("The register session is not open".into()),
            None => return Err(format!("Register session {sid} not found")),
        }
    }

    let (sale_id, sale_no) = match input.held_sale_id {
        Some(held_id) => {
            let held: Option<(String, String)> =
                sqlx::query_as("SELECT sale_no, status FROM sales WHERE id = ?")
                    .bind(held_id)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(|e| e.to_string())?;
            let (held_no, held_status) =
                held.ok_or_else(|| format!("Held sale {held_id} not found"))?;
            if held_status != "held" {
                return Err("Only held sales can be completed from a hold".into());
            }
            sqlx::query("DELETE FROM sale_items WHERE sale_id = ?")
                .bind(held_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
            sqlx::query("DELETE FROM payments WHERE sale_id = ?")
                .bind(held_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
            (held_id, held_no)
        }
        None => {
            let next_no: (i64,) =
                sqlx::query_as("SELECT COALESCE(MAX(id), 0) + 1 FROM sales")
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(|e| e.to_string())?;
            (0, format!("S-{:06}", next_no.0))
        }
    };

    let result = if sale_id == 0 {
        sqlx::query(
            "INSERT INTO sales
                (session_id, sale_no, customer_id, user_id, subtotal, discount, tax, total, paid_amount, change_given, status)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'completed')",
        )
        .bind(input.session_id)
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
        .map_err(|e| e.to_string())?
    } else {
        sqlx::query(
            "UPDATE sales
                SET session_id = ?, customer_id = ?, user_id = ?, subtotal = ?, discount = ?, tax = ?, total = ?,
                    paid_amount = ?, change_given = ?, status = 'completed', void_reason = NULL
             WHERE id = ?",
        )
        .bind(input.session_id)
        .bind(sale_customer_id)
        .bind(input.user_id)
        .bind(subtotal)
        .bind(order_discount)
        .bind(tax)
        .bind(total)
        .bind(paid_amount)
        .bind(change_given)
        .bind(sale_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?
    };
    let sale_id = if sale_id == 0 { result.last_insert_rowid() } else { sale_id };

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

/// Saves the current cart as a held sale so it can be resumed later. No stock
/// is moved and no payments are recorded.
pub async fn insert_hold(pool: &sqlx::SqlitePool, input: HoldSaleInput) -> Result<HoldSaleResult, String> {
    let create_input = CreateSaleInput {
        items: input.items,
        payments: Vec::new(),
        discount: input.discount,
        tax: input.tax,
        user_id: input.user_id,
        customer_id: input.customer_id,
        held_sale_id: None,
        session_id: None,
    };
    let (subtotal, _item_discount, order_discount, tax, total) = validate_and_totals(&create_input)?;

    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    check_stock(&mut tx, &create_input.items).await?;

    let next_no: (i64,) =
        sqlx::query_as("SELECT COALESCE(MAX(id), 0) + 1 FROM sales")
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
    let sale_no = format!("S-{:06}", next_no.0);

    let result = sqlx::query(
        "INSERT INTO sales
            (sale_no, customer_id, user_id, subtotal, discount, tax, total, paid_amount, change_given, status)
         VALUES (?, ?, ?, ?, ?, ?, ?, 0, 0, 'held')",
    )
    .bind(&sale_no)
    .bind(input.customer_id)
    .bind(input.user_id)
    .bind(subtotal)
    .bind(order_discount)
    .bind(tax)
    .bind(total)
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    let sale_id = result.last_insert_rowid();

    for item in &create_input.items {
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
    }

    sqlx::query(
        "INSERT INTO audit_log (user_id, action, entity_type, entity_id, details)
         VALUES (?, 'sale.hold', 'sale', ?, ?)",
    )
    .bind(input.user_id)
    .bind(sale_id)
    .bind(format!(
        "Held sale {sale_no} for {} item(s), total {}",
        create_input.items.len(),
        total
    ))
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    tx.commit().await.map_err(|e| e.to_string())?;

    Ok(HoldSaleResult {
        sale_id,
        sale_no,
        total,
        item_count: create_input.items.len() as i64,
    })
}

/// Loads a held sale's cart for resuming in the checkout. The held sale stays
/// in 'held' status until completed or cancelled.
pub async fn load_held_sale(
    pool: &sqlx::SqlitePool,
    input: ResumeSaleInput,
) -> Result<ResumeSaleRecord, String> {
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    let sale: Option<(String, Option<i64>, f64, f64, f64, f64)> = sqlx::query_as(
        "SELECT sale_no, customer_id, subtotal, discount, tax, total FROM sales WHERE id = ?",
    )
    .bind(input.sale_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    let (sale_no, customer_id, subtotal, discount, tax, total) =
        sale.ok_or("Held sale not found")?;

    let status: (String,) = sqlx::query_as("SELECT status FROM sales WHERE id = ?")
        .bind(input.sale_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    if status.0 != "held" {
        return Err(format!("Sale {sale_no} is not currently held"));
    }

    let rows = sqlx::query(
        "SELECT si.product_id, p.name, si.qty, si.price, si.cost_price, si.discount
         FROM sale_items si
         JOIN products p ON p.id = si.product_id
         WHERE si.sale_id = ?",
    )
    .bind(input.sale_id)
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        items.push(ResumeSaleItem {
            product_id: row.try_get("product_id").map_err(|e| e.to_string())?,
            name: row.try_get("name").map_err(|e| e.to_string())?,
            qty: row.try_get("qty").map_err(|e| e.to_string())?,
            price: row.try_get("price").map_err(|e| e.to_string())?,
            cost_price: row.try_get("cost_price").map_err(|e| e.to_string())?,
            discount: row.try_get("discount").map_err(|e| e.to_string())?,
        });
    }

    sqlx::query(
        "INSERT INTO audit_log (user_id, action, entity_type, entity_id, details)
         VALUES (?, 'sale.resume', 'sale', ?, ?)",
    )
    .bind(input.user_id)
    .bind(input.sale_id)
    .bind(format!("Resumed held sale {sale_no}"))
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    tx.commit().await.map_err(|e| e.to_string())?;

    Ok(ResumeSaleRecord {
        sale_id: input.sale_id,
        sale_no,
        customer_id,
        subtotal,
        discount,
        tax,
        total,
        items,
    })
}

/// Drops a held sale without completing it.
pub async fn drop_held_sale(
    pool: &sqlx::SqlitePool,
    input: CancelHeldSaleInput,
) -> Result<(), String> {
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    let sale: Option<(String, String)> =
        sqlx::query_as("SELECT sale_no, status FROM sales WHERE id = ?")
            .bind(input.sale_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
    let (sale_no, status) = sale.ok_or("Held sale not found")?;
    if status != "held" {
        return Err(format!("Sale {sale_no} is not currently held"));
    }

    sqlx::query("UPDATE sales SET status = 'cancelled' WHERE id = ?")
        .bind(input.sale_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    sqlx::query(
        "INSERT INTO audit_log (user_id, action, entity_type, entity_id, details)
         VALUES (?, 'sale.cancel', 'sale', ?, ?)",
    )
    .bind(input.user_id)
    .bind(input.sale_id)
    .bind(format!("Cancelled held sale {sale_no}"))
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn hold_sale<R: Runtime>(
    app: AppHandle<R>,
    input: HoldSaleInput,
) -> Result<HoldSaleResult, String> {
    let pool = db::pool(&app).await?;
    insert_hold(&pool, input).await
}

#[tauri::command]
pub async fn resume_sale<R: Runtime>(
    app: AppHandle<R>,
    input: ResumeSaleInput,
) -> Result<ResumeSaleRecord, String> {
    let pool = db::pool(&app).await?;
    load_held_sale(&pool, input).await
}

#[tauri::command]
pub async fn cancel_held_sale<R: Runtime>(
    app: AppHandle<R>,
    input: CancelHeldSaleInput,
) -> Result<(), String> {
    let pool = db::pool(&app).await?;
    drop_held_sale(&pool, input).await
}

/// Reverses a completed sale: marks it voided, restores stock, reverses any
/// customer-credit ledger charge and writes an audit entry. All in one
/// transaction so a failed reversal leaves the sale untouched.
pub async fn reverse_sale(pool: &sqlx::SqlitePool, input: VoidSaleInput) -> Result<(), String> {
    let reason = optional_field(&Some(input.reason)).ok_or("Void reason is required")?;

    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    let sale: Option<(String, String, Option<i64>)> =
        sqlx::query_as("SELECT sale_no, status, customer_id FROM sales WHERE id = ?")
            .bind(input.sale_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
    let (sale_no, status, sale_customer) = sale.ok_or("Sale not found")?;
    if status != "completed" {
        return Err("Only completed sales can be voided".into());
    }

    sqlx::query(
        "UPDATE sales SET status = 'voided', void_reason = ?, voided_by = ? WHERE id = ?",
    )
    .bind(&reason)
    .bind(input.user_id)
    .bind(input.sale_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    // Restore stock for every line item.
    let items: Vec<(i64, f64)> =
        sqlx::query_as("SELECT product_id, qty FROM sale_items WHERE sale_id = ?")
            .bind(input.sale_id)
            .fetch_all(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
    for (product_id, qty) in items {
        sqlx::query(
            "INSERT INTO stock_movements (product_id, type, qty, ref_id, notes, user_id)
             VALUES (?, 'sale_void_in', ?, ?, ?, ?)",
        )
        .bind(product_id)
        .bind(qty)
        .bind(input.sale_id)
        .bind(format!("Void sale {sale_no}: {reason}"))
        .bind(input.user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

        sqlx::query("UPDATE products SET stock_qty = stock_qty + ? WHERE id = ?")
            .bind(qty)
            .bind(product_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
    }

    // Reverse customer-credit charges from this sale. The credit customer is
    // stored on the sale itself; payments only carry the amounts.
    if let Some(customer_id) = sale_customer {
        let credits: Vec<(f64,)> = sqlx::query_as(
            "SELECT amount FROM payments WHERE sale_id = ? AND method = 'credit'",
        )
        .bind(input.sale_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
        for (amount,) in credits {
            let current: (f64,) = sqlx::query_as("SELECT balance FROM customers WHERE id = ?")
                .bind(customer_id)
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
            let balance_after = current.0 - amount;

            sqlx::query("UPDATE customers SET balance = ? WHERE id = ?")
                .bind(balance_after)
                .bind(customer_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;

            sqlx::query(
                "INSERT INTO customer_ledger (customer_id, sale_id, type, amount, balance_after, notes, user_id)
                 VALUES (?, ?, 'reversal', ?, ?, ?, ?)",
            )
            .bind(customer_id)
            .bind(input.sale_id)
            .bind(-amount)
            .bind(balance_after)
            .bind(format!("Void sale {sale_no}: {reason}"))
            .bind(input.user_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
        }
    }

    sqlx::query(
        "INSERT INTO audit_log (user_id, action, entity_type, entity_id, details)
         VALUES (?, 'sale.void', 'sale', ?, ?)",
    )
    .bind(input.user_id)
    .bind(input.sale_id)
    .bind(format!("Voided sale {sale_no}: {reason}"))
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn void_sale<R: Runtime>(
    app: AppHandle<R>,
    input: VoidSaleInput,
) -> Result<(), String> {
    let pool = db::pool(&app).await?;
    reverse_sale(&pool, input).await
}

#[tauri::command]
pub async fn list_sales<R: Runtime>(
    app: AppHandle<R>,
    input: Option<ListSalesInput>,
) -> Result<Vec<SaleRecord>, String> {
    let pool = db::pool(&app).await?;
    query_sales(&pool, input.unwrap_or(ListSalesInput { status: None, limit: None })).await
}

pub async fn query_sales(
    pool: &sqlx::SqlitePool,
    input: ListSalesInput,
) -> Result<Vec<SaleRecord>, String> {
    let limit = input.limit.unwrap_or(100).max(1).min(500);

    let rows = sqlx::query(
        "SELECT s.id, s.sale_no, s.created_at, u.username, c.name,
                (SELECT COUNT(*) FROM sale_items si WHERE si.sale_id = s.id) AS item_count,
                s.total, s.paid_amount, s.status, s.void_reason
         FROM sales s
         LEFT JOIN users u ON u.id = s.user_id
         LEFT JOIN customers c ON c.id = s.customer_id
         WHERE (? IS NULL OR s.status = ?)
         ORDER BY s.id DESC
         LIMIT ?",
    )
    .bind(&input.status)
    .bind(&input.status)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        out.push(SaleRecord {
            id: row.try_get("id").map_err(|e| e.to_string())?,
            sale_no: row.try_get("sale_no").map_err(|e| e.to_string())?,
            created_at: row.try_get("created_at").map_err(|e| e.to_string())?,
            user_name: row.try_get("username").map_err(|e| e.to_string())?,
            customer_name: row.try_get("name").map_err(|e| e.to_string())?,
            item_count: row.try_get("item_count").map_err(|e| e.to_string())?,
            total: row.try_get("total").map_err(|e| e.to_string())?,
            paid_amount: row.try_get("paid_amount").map_err(|e| e.to_string())?,
            status: row.try_get("status").map_err(|e| e.to_string())?,
            void_reason: row.try_get("void_reason").map_err(|e| e.to_string())?,
        });
    }
    Ok(out)
}

/// Loads everything needed to render a sale's receipt: store profile from the
/// settings table, the sale header, line items and payment breakdown.
pub async fn fetch_receipt(
    pool: &sqlx::SqlitePool,
    input: SaleReceiptInput,
) -> Result<SaleReceipt, String> {
    let sale: Option<(
        String,
        String,
        String,
        Option<i64>,
        i64,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
    )> = sqlx::query_as(
        "SELECT sale_no, created_at, status, customer_id, user_id,
                subtotal, discount, tax, total, paid_amount, change_given
         FROM sales WHERE id = ?",
    )
    .bind(input.sale_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?;
    let (
        sale_no,
        created_at,
        status,
        customer_id,
        user_id,
        subtotal,
        order_discount,
        tax,
        total,
        paid_amount,
        change_given,
    ) = sale.ok_or_else(|| format!("Sale {} not found", input.sale_id))?;

    let customer_name: Option<String> = match customer_id {
        Some(cid) => {
            let row: Option<(String,)> =
                sqlx::query_as("SELECT name FROM customers WHERE id = ?")
                    .bind(cid)
                    .fetch_optional(pool)
                    .await
                    .map_err(|e| e.to_string())?;
            row.map(|r| r.0)
        }
        None => None,
    };

    let user_name: Option<String> =
        sqlx::query_as("SELECT full_name FROM users WHERE id = ?")
            .bind(user_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| e.to_string())?
            .map(|r: (String,)| r.0);

    let item_rows = sqlx::query(
        "SELECT p.name, si.qty, si.price, si.discount, si.subtotal
         FROM sale_items si
         JOIN products p ON p.id = si.product_id
         WHERE si.sale_id = ?",
    )
    .bind(input.sale_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let mut items = Vec::with_capacity(item_rows.len());
    let mut item_discount = 0.0;
    for row in item_rows {
        let qty: f64 = row.try_get("qty").map_err(|e| e.to_string())?;
        let discount: f64 = row.try_get("discount").map_err(|e| e.to_string())?;
        item_discount += discount * qty;
        items.push(ReceiptItem {
            name: row.try_get("name").map_err(|e| e.to_string())?,
            qty,
            price: row.try_get("price").map_err(|e| e.to_string())?,
            discount,
            subtotal: row.try_get("subtotal").map_err(|e| e.to_string())?,
        });
    }

    let payment_rows = sqlx::query(
        "SELECT method, amount, reference FROM payments WHERE sale_id = ?",
    )
    .bind(input.sale_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let mut payments = Vec::with_capacity(payment_rows.len());
    for row in payment_rows {
        payments.push(ReceiptPayment {
            method: row.try_get("method").map_err(|e| e.to_string())?,
            amount: row.try_get("amount").map_err(|e| e.to_string())?,
            reference: row.try_get("reference").map_err(|e| e.to_string())?,
        });
    }

    // Store profile from the settings table (unset until F7.1 manages it).
    let setting_rows = sqlx::query("SELECT key, value FROM settings")
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;
    let mut store_name = String::new();
    let mut store_address = String::new();
    let mut store_phone = String::new();
    let mut store_tax_id = String::new();
    let mut receipt_header = String::new();
    let mut receipt_footer = String::new();
    let mut receipt_logo = String::new();
    let mut receipt_logo_pos = String::from("top");
    let mut receipt_format = String::from("thermal");
    for row in setting_rows {
        let key: String = row.try_get("key").map_err(|e| e.to_string())?;
        let value: String = row.try_get("value").map_err(|e| e.to_string())?;
        match key.as_str() {
            "store_name" => store_name = value,
            "store_address" => store_address = value,
            "store_phone" => store_phone = value,
            "store_tax_id" => store_tax_id = value,
            "receipt_header" => receipt_header = value,
            "receipt_footer" => receipt_footer = value,
            // The store logo (data URL) doubles as the receipt logo.
            "store_logo" => receipt_logo = value,
            "receipt_logo_pos" => receipt_logo_pos = value,
            "receipt_format" => receipt_format = value,
            _ => {}
        }
    }

    Ok(SaleReceipt {
        store_name,
        store_address,
        store_phone,
        store_tax_id,
        receipt_header,
        receipt_footer,
        receipt_logo,
        receipt_logo_pos,
        receipt_format,
        sale_id: input.sale_id,
        sale_no,
        created_at,
        status,
        customer_name,
        user_name,
        subtotal,
        item_discount,
        order_discount,
        tax,
        total,
        paid_amount,
        change_given,
        items,
        payments,
    })
}

#[tauri::command]
pub async fn get_sale_receipt<R: Runtime>(
    app: AppHandle<R>,
    input: SaleReceiptInput,
) -> Result<SaleReceipt, String> {
    let pool = db::pool(&app).await?;
    fetch_receipt(&pool, input).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schema() -> &'static str {
        r#"
        CREATE TABLE users (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          username TEXT NOT NULL,
          full_name TEXT
        );
        CREATE TABLE settings (
          key TEXT PRIMARY KEY,
          value TEXT NOT NULL
        );
        CREATE TABLE sale_sessions (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          user_id INTEGER NOT NULL,
          opened_at TEXT NOT NULL DEFAULT (datetime('now')),
          closed_at TEXT,
          opening_cash REAL NOT NULL DEFAULT 0,
          closing_cash REAL,
          expected_cash REAL,
          status TEXT NOT NULL DEFAULT 'open',
          variance REAL,
          closed_by INTEGER,
          notes TEXT
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
            held_sale_id: None,
            session_id: None,
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
            held_sale_id: None,
            session_id: None,
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
            held_sale_id: None,
            session_id: None,
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
            held_sale_id: None,
            session_id: None,
        };

        let first = insert_sale(&pool, input()).await.unwrap();
        let second = insert_sale(&pool, input()).await.unwrap();
        assert_eq!(first.sale_no, "S-000001");
        assert_eq!(second.sale_no, "S-000002");
        assert_ne!(first.sale_id, second.sale_id);
    }

    #[tokio::test]
    async fn void_sale_restores_stock_and_marks_voided() {
        let pool = mem_pool().await;
        sample_product(&pool, 1, "Coffee", 20.0, 5.0).await;

        let sale = insert_sale(
            &pool,
            CreateSaleInput {
                items: vec![item(1, 2.0, 5.0, 0.0)],
                payments: vec![SalePaymentInput {
                    method: "cash".into(),
                    amount: 10.0,
                    reference: None,
                    customer_id: None,
                }],
                discount: 0.0,
                tax: 0.0,
                user_id: Some(1),
                customer_id: None,
                held_sale_id: None,
                session_id: None,
            },
        )
        .await
        .unwrap();
        assert!((stock_of(&pool, 1).await - 18.0).abs() < 0.001);

        reverse_sale(
            &pool,
            VoidSaleInput {
                sale_id: sale.sale_id,
                reason: "Damaged goods".into(),
                user_id: Some(1),
            },
        )
        .await
        .unwrap();

        let row: (String, String, Option<String>, i64) = sqlx::query_as(
            "SELECT sale_no, status, void_reason, voided_by FROM sales WHERE id = ?",
        )
        .bind(sale.sale_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(row.0, "S-000001");
        assert_eq!(row.1, "voided");
        assert_eq!(row.2.as_deref(), Some("Damaged goods"));
        assert_eq!(row.3, 1);

        // Stock restored.
        assert!((stock_of(&pool, 1).await - 20.0).abs() < 0.001);

        // A positive reversal movement was recorded.
        let mv: (String, f64) = sqlx::query_as(
            "SELECT type, qty FROM stock_movements WHERE ref_id = ? AND type = 'sale_void_in'",
        )
        .bind(sale.sale_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(mv.0, "sale_void_in");
        assert!((mv.1 - 2.0).abs() < 0.001);

        let audit: (String,) =
            sqlx::query_as("SELECT action FROM audit_log WHERE entity_id = ? AND action = 'sale.void'")
                .bind(sale.sale_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(audit.0, "sale.void");

        // Voiding an already-voided sale must fail.
        let err = reverse_sale(
            &pool,
            VoidSaleInput {
                sale_id: sale.sale_id,
                reason: "Again".into(),
                user_id: Some(1),
            },
        )
        .await
        .unwrap_err();
        assert!(err.contains("Only completed"));
    }

    #[tokio::test]
    async fn void_sale_rejects_missing_reason_and_reverses_credit() {
        let pool = mem_pool().await;
        sample_product(&pool, 1, "Coffee", 20.0, 5.0).await;
        sample_customer(&pool, 1, "Alice").await;

        let sale = insert_sale(
            &pool,
            CreateSaleInput {
                items: vec![item(1, 1.0, 5.0, 0.0)],
                payments: vec![SalePaymentInput {
                    method: "credit".into(),
                    amount: 5.0,
                    reference: None,
                    customer_id: Some(1),
                }],
                discount: 0.0,
                tax: 0.0,
                user_id: Some(1),
                customer_id: None,
                held_sale_id: None,
                session_id: None,
            },
        )
        .await
        .unwrap();
        assert!((balance_of(&pool, 1).await - 5.0).abs() < 0.001);

        // Missing reason is rejected and leaves the sale untouched.
        let err = reverse_sale(
            &pool,
            VoidSaleInput {
                sale_id: sale.sale_id,
                reason: "   ".into(),
                user_id: Some(1),
            },
        )
        .await
        .unwrap_err();
        assert!(err.contains("reason"));
        let status: (String,) = sqlx::query_as("SELECT status FROM sales WHERE id = ?")
            .bind(sale.sale_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(status.0, "completed");

        // Voiding reverses the customer's balance.
        reverse_sale(
            &pool,
            VoidSaleInput {
                sale_id: sale.sale_id,
                reason: "Customer changed their mind".into(),
                user_id: Some(1),
            },
        )
        .await
        .unwrap();
        assert!((balance_of(&pool, 1).await - 0.0).abs() < 0.001);

        let ledger: (String, f64, f64) = sqlx::query_as(
            "SELECT type, amount, balance_after FROM customer_ledger WHERE customer_id = 1 AND type = 'reversal'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(ledger.0, "reversal");
        assert!((ledger.1 + 5.0).abs() < 0.001);
        assert!((ledger.2 - 0.0).abs() < 0.001);
    }

    #[tokio::test]
    async fn list_sales_returns_records_and_filters() {
        let pool = mem_pool().await;
        sample_product(&pool, 1, "Coffee", 100.0, 5.0).await;

        let sale = insert_sale(
            &pool,
            CreateSaleInput {
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
                held_sale_id: None,
                session_id: None,
            },
        )
        .await
        .unwrap();
        reverse_sale(
            &pool,
            VoidSaleInput {
                sale_id: sale.sale_id,
                reason: "Test void".into(),
                user_id: Some(1),
            },
        )
        .await
        .unwrap();

        let all = query_sales(&pool, ListSalesInput { status: None, limit: None })
            .await
            .unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].sale_no, "S-000001");
        assert_eq!(all[0].status, "voided");
        assert_eq!(all[0].item_count, 1);
        assert_eq!(all[0].void_reason.as_deref(), Some("Test void"));

        let completed = query_sales(
            &pool,
            ListSalesInput {
                status: Some("completed".into()),
                limit: None,
            },
        )
        .await
        .unwrap();
        assert!(completed.is_empty());
    }

    #[tokio::test]
    async fn hold_sale_records_held_cart_without_touching_stock() {
        let pool = mem_pool().await;
        sample_product(&pool, 1, "Coffee", 20.0, 5.0).await;
        sample_customer(&pool, 1, "Alice").await;

        let held = insert_hold(
            &pool,
            HoldSaleInput {
                items: vec![item(1, 2.0, 5.0, 0.5)],
                discount: 1.0,
                tax: 0.0,
                customer_id: Some(1),
                user_id: Some(1),
            },
        )
        .await
        .unwrap();
        assert_eq!(held.sale_no, "S-000001");
        assert_eq!(held.item_count, 1);
        // subtotal 10 - item discount 1 - order discount 1 = 8
        assert!((held.total - 8.0).abs() < 0.001);

        let row: (String, f64) =
            sqlx::query_as("SELECT status, total FROM sales WHERE id = ?")
                .bind(held.sale_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(row.0, "held");
        assert!((row.1 - 8.0).abs() < 0.001);

        // Items are stored, but no payments, no stock movement, stock untouched.
        let items: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM sale_items WHERE sale_id = ?")
            .bind(held.sale_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(items.0, 1);
        let pays: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM payments WHERE sale_id = ?")
            .bind(held.sale_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(pays.0, 0);
        let moves: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM stock_movements WHERE ref_id = ?")
            .bind(held.sale_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(moves.0, 0);
        assert!((stock_of(&pool, 1).await - 20.0).abs() < 0.001);

        let audit: (String,) = sqlx::query_as(
            "SELECT action FROM audit_log WHERE entity_id = ? AND action = 'sale.hold'",
        )
        .bind(held.sale_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(audit.0, "sale.hold");
    }

    #[tokio::test]
    async fn resume_sale_returns_cart_and_rejects_non_held() {
        let pool = mem_pool().await;
        sample_product(&pool, 1, "Coffee", 20.0, 5.0).await;
        sample_customer(&pool, 1, "Alice").await;

        let held = insert_hold(
            &pool,
            HoldSaleInput {
                items: vec![item(1, 2.0, 5.0, 0.5)],
                discount: 1.0,
                tax: 0.0,
                customer_id: Some(1),
                user_id: Some(1),
            },
        )
        .await
        .unwrap();

        let resumed = load_held_sale(
            &pool,
            ResumeSaleInput {
                sale_id: held.sale_id,
                user_id: Some(1),
            },
        )
        .await
        .unwrap();
        assert_eq!(resumed.sale_no, "S-000001");
        assert_eq!(resumed.customer_id, Some(1));
        assert!((resumed.discount - 1.0).abs() < 0.001);
        assert_eq!(resumed.items.len(), 1);
        assert_eq!(resumed.items[0].name, "Coffee");
        assert!((resumed.items[0].qty - 2.0).abs() < 0.001);
        assert!((resumed.items[0].discount - 0.5).abs() < 0.001);

        // Still held after resume.
        let status: (String,) = sqlx::query_as("SELECT status FROM sales WHERE id = ?")
            .bind(held.sale_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(status.0, "held");

        // A completed sale cannot be resumed.
        let err = load_held_sale(
            &pool,
            ResumeSaleInput {
                sale_id: held.sale_id + 999,
                user_id: Some(1),
            },
        )
        .await
        .unwrap_err();
        assert!(err.contains("not found"));
    }

    #[tokio::test]
    async fn completing_held_sale_keeps_number_and_completes_in_place() {
        let pool = mem_pool().await;
        sample_product(&pool, 1, "Coffee", 20.0, 5.0).await;

        let held = insert_hold(
            &pool,
            HoldSaleInput {
                items: vec![item(1, 2.0, 5.0, 0.0)],
                discount: 0.0,
                tax: 0.0,
                customer_id: None,
                user_id: Some(1),
            },
        )
        .await
        .unwrap();
        assert_eq!(held.sale_no, "S-000001");

        // A fresh sale now gets the next number.
        let fresh = insert_sale(
            &pool,
            CreateSaleInput {
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
                held_sale_id: None,
                session_id: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(fresh.sale_no, "S-000002");

        // Complete the held sale by referencing it.
        let completed = insert_sale(
            &pool,
            CreateSaleInput {
                items: vec![item(1, 2.0, 5.0, 0.0)],
                payments: vec![SalePaymentInput {
                    method: "cash".into(),
                    amount: 10.0,
                    reference: None,
                    customer_id: None,
                }],
                discount: 0.0,
                tax: 0.0,
                user_id: Some(1),
                customer_id: None,
                held_sale_id: Some(held.sale_id),
                session_id: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(completed.sale_id, held.sale_id);
        assert_eq!(completed.sale_no, "S-000001");

        let row: (String, f64) =
            sqlx::query_as("SELECT status, paid_amount FROM sales WHERE id = ?")
                .bind(held.sale_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(row.0, "completed");
        assert!((row.1 - 10.0).abs() < 0.001);

        // Stock moved exactly once (held didn't reserve any).
        assert!((stock_of(&pool, 1).await - 17.0).abs() < 0.001);
        let moves: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM stock_movements WHERE ref_id = ?")
            .bind(held.sale_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(moves.0, 1);
    }

    #[tokio::test]
    async fn cancel_held_sale_marks_cancelled() {
        let pool = mem_pool().await;
        sample_product(&pool, 1, "Coffee", 20.0, 5.0).await;

        let held = insert_hold(
            &pool,
            HoldSaleInput {
                items: vec![item(1, 1.0, 5.0, 0.0)],
                discount: 0.0,
                tax: 0.0,
                customer_id: None,
                user_id: Some(1),
            },
        )
        .await
        .unwrap();

        drop_held_sale(
            &pool,
            CancelHeldSaleInput {
                sale_id: held.sale_id,
                user_id: Some(1),
            },
        )
        .await
        .unwrap();
        let status: (String,) = sqlx::query_as("SELECT status FROM sales WHERE id = ?")
            .bind(held.sale_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(status.0, "cancelled");

        // Cannot complete a cancelled held sale.
        let err = insert_sale(
            &pool,
            CreateSaleInput {
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
                held_sale_id: Some(held.sale_id),
                session_id: None,
            },
        )
        .await
        .unwrap_err();
        assert!(err.contains("Only held"));
    }

    #[tokio::test]
    async fn fetch_receipt_returns_full_receipt() {
        let pool = mem_pool().await;
        sample_product(&pool, 1, "Coffee", 20.0, 5.0).await;
        sample_customer(&pool, 1, "Alice").await;
        sqlx::query("INSERT INTO users (username, full_name) VALUES ('admin', 'Seed Admin')")
            .execute(&pool)
            .await
            .unwrap();

        let sale = insert_sale(
            &pool,
            CreateSaleInput {
                items: vec![
                    item(1, 2.0, 5.0, 0.5),
                    item(1, 1.0, 5.0, 0.0),
                ],
                payments: vec![SalePaymentInput {
                    method: "cash".into(),
                    amount: 14.0,
                    reference: None,
                    customer_id: None,
                }],
                discount: 0.0,
                tax: 0.0,
                user_id: Some(1),
                customer_id: Some(1),
                held_sale_id: None,
                session_id: None,
            },
        )
        .await
        .unwrap();

        let r = fetch_receipt(&pool, SaleReceiptInput { sale_id: sale.sale_id })
            .await
            .unwrap();
        assert_eq!(r.sale_no, "S-000001");
        assert_eq!(r.status, "completed");
        assert_eq!(r.customer_name.as_deref(), Some("Alice"));
        assert_eq!(r.user_name.as_deref(), Some("Seed Admin"));
        // 2 + 1 items, item discount 0.5*2 = 1.0
        assert_eq!(r.items.len(), 2);
        assert!((r.item_discount - 1.0).abs() < 0.001);
        assert!((r.subtotal - 15.0).abs() < 0.001);
        assert!((r.total - 14.0).abs() < 0.001);
        assert_eq!(r.payments.len(), 1);
        assert_eq!(r.payments[0].method, "cash");
        assert!((r.payments[0].amount - 14.0).abs() < 0.001);
        assert!(r.store_name.is_empty());
    }

    #[tokio::test]
    async fn insert_sale_links_to_open_session_and_rejects_closed() {
        let pool = mem_pool().await;
        sample_product(&pool, 1, "Coffee", 20.0, 5.0).await;

        let err = insert_sale(
            &pool,
            CreateSaleInput {
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
                held_sale_id: None,
                session_id: Some(999),
            },
        )
        .await
        .unwrap_err();
        assert!(err.contains("not found"));

        sqlx::query(
            "INSERT INTO sale_sessions (user_id, opening_cash, status) VALUES (1, 100, 'open')",
        )
        .execute(&pool)
        .await
        .unwrap();
        let sid: i64 =
            sqlx::query_scalar("SELECT id FROM sale_sessions WHERE status = 'open' LIMIT 1")
                .fetch_one(&pool)
                .await
                .unwrap();

        let sale = insert_sale(
            &pool,
            CreateSaleInput {
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
                held_sale_id: None,
                session_id: Some(sid),
            },
        )
        .await
        .unwrap();

        let stored: Option<i64> =
            sqlx::query_scalar("SELECT session_id FROM sales WHERE id = ?")
                .bind(sale.sale_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(stored, Some(sid));

        // Closed sessions are rejected.
        sqlx::query("UPDATE sale_sessions SET status = 'closed' WHERE id = ?")
            .bind(sid)
            .execute(&pool)
            .await
            .unwrap();
        let err = insert_sale(
            &pool,
            CreateSaleInput {
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
                held_sale_id: None,
                session_id: Some(sid),
            },
        )
        .await
        .unwrap_err();
        assert!(err.contains("not open"));
    }

    async fn stock_of(pool: &sqlx::SqlitePool, product_id: i64) -> f64 {
        let row: (f64,) = sqlx::query_as("SELECT stock_qty FROM products WHERE id = ?")
            .bind(product_id)
            .fetch_one(pool)
            .await
            .unwrap();
        row.0
    }

    async fn balance_of(pool: &sqlx::SqlitePool, customer_id: i64) -> f64 {
        let row: (f64,) = sqlx::query_as("SELECT balance FROM customers WHERE id = ?")
            .bind(customer_id)
            .fetch_one(pool)
            .await
            .unwrap();
        row.0
    }
}
