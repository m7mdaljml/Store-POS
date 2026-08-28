use serde::Serialize;
use tauri::{AppHandle, Runtime};

use crate::db;

/// Shared period filter fragment: completed sales within [from, to] dates.
// created_at is stored as UTC (datetime('now')); convert to localtime so the
// report's local-date range (from/to, e.g. "today") matches the store clock.
const SALES_PERIOD_FILTER: &str =
    "s.status = 'completed' AND date(s.created_at, 'localtime') BETWEEN ?1 AND ?2";

/// Net invoice value: original total minus every refund issued against the sale.
/// Requires the sales table to be aliased as `s`.
const NET_SALE_TOTAL: &str =
    "(s.total - COALESCE((SELECT SUM(r.amount) FROM refunds r WHERE r.sale_id = s.id), 0.0))";

/// Net units sold on a line: original qty minus what was already returned.
/// Requires the sale_items table to be aliased as `si`.
const NET_ITEM_QTY: &str = "(si.qty - si.refunded_qty)";

// TOTAL() (unlike SUM) always returns REAL — even on empty sets — so sqlx can
// decode the value into f64 without an INTEGER/REAL mismatch.

fn validate_range(from: &str, to: &str) -> Result<(), String> {
    if from.len() != 10 || to.len() != 10 {
        return Err("Dates must be in YYYY-MM-DD format".into());
    }
    Ok(())
}

// ============================================================
// F6.2 Summary
// ============================================================

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SalesSummary {
    pub revenue: f64,
    pub orders: i64,
    pub avg_ticket: f64,
    pub gross_profit: f64,
    pub expenses_total: f64,
    pub net_position: f64,
}

pub async fn compute_summary(
    pool: &sqlx::SqlitePool,
    from: &str,
    to: &str,
) -> Result<SalesSummary, String> {
    let agg: (f64, i64) = sqlx::query_as(
        "SELECT TOTAL(s.total - COALESCE((SELECT SUM(r.amount) FROM refunds r WHERE r.sale_id = s.id), 0.0)), COUNT(*) FROM sales s WHERE s.status = 'completed' AND date(s.created_at, 'localtime') BETWEEN ?1 AND ?2",
    )
    .bind(from)
    .bind(to)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;

    let profit: (f64,) = sqlx::query_as(&format!(
        "SELECT TOTAL((si.price - si.discount - si.cost_price) * {NET_ITEM_QTY})
         FROM sale_items si JOIN sales s ON s.id = si.sale_id WHERE s.status = 'completed' AND date(s.created_at, 'localtime') BETWEEN ?1 AND ?2"
    ))
    .bind(from)
    .bind(to)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;

    let expenses: (f64,) =
        sqlx::query_as("SELECT TOTAL(amount) FROM expense_out WHERE date(date) BETWEEN ?1 AND ?2")
            .bind(from)
            .bind(to)
            .fetch_one(pool)
            .await
            .map_err(|e| e.to_string())?;

    let revenue = agg.0;
    let orders = agg.1;
    Ok(SalesSummary {
        revenue,
        orders,
        avg_ticket: if orders > 0 { revenue / orders as f64 } else { 0.0 },
        gross_profit: profit.0,
        expenses_total: expenses.0,
        net_position: revenue - expenses.0,
    })
}

#[tauri::command]
pub async fn sales_summary<R: Runtime>(
    app: AppHandle<R>,
    from: String,
    to: String,
) -> Result<SalesSummary, String> {
    validate_range(&from, &to)?;
    let pool = db::pool(&app).await?;
    compute_summary(&pool, &from, &to).await
}

// ============================================================
// F6.5 Revenue trend
// ============================================================

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrendPoint {
    pub bucket: String,
    pub revenue: f64,
    pub orders: i64,
}

fn bucket_expr(granularity: &str) -> Result<&'static str, String> {
    match granularity {
        "day" => Ok("strftime('%Y-%m-%d', s.created_at, 'localtime')"),
        "week" => Ok("strftime('%Y-W%W', s.created_at, 'localtime')"),
        "month" => Ok("strftime('%Y-%m', s.created_at, 'localtime')"),
        other => Err(format!("Unknown granularity '{other}'")),
    }
}

pub(crate) async fn compute_trend(
    pool: &sqlx::SqlitePool,
    from: &str,
    to: &str,
    granularity: &str,
) -> Result<Vec<TrendPoint>, String> {
    let expr = bucket_expr(granularity)?;
    let sql = format!(
        "SELECT {expr} AS bucket, TOTAL({NET_SALE_TOTAL}) AS revenue, COUNT(*) AS orders
         FROM sales s WHERE {SALES_PERIOD_FILTER}
         GROUP BY bucket
         HAVING TOTAL({NET_SALE_TOTAL}) > 0.005
         ORDER BY bucket"
    );
    let rows: Vec<(String, f64, i64)> = sqlx::query_as(&sql)
        .bind(from)
        .bind(to)
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(rows
        .into_iter()
        .map(|(bucket, revenue, orders)| TrendPoint { bucket, revenue, orders })
        .collect())
}

#[tauri::command]
pub async fn revenue_trend<R: Runtime>(
    app: AppHandle<R>,
    from: String,
    to: String,
    granularity: Option<String>,
) -> Result<Vec<TrendPoint>, String> {
    validate_range(&from, &to)?;
    let pool = db::pool(&app).await?;
    compute_trend(&pool, &from, &to, granularity.as_deref().unwrap_or("day")).await
}

// ============================================================
// F6.4 Top sellers + category breakdown
// ============================================================

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TopProductRow {
    pub product_id: i64,
    pub name: String,
    pub category: Option<String>,
    pub qty: f64,
    pub revenue: f64,
    pub profit: f64,
}

fn top_products_sql(limit: Option<i64>) -> String {
    let limit_clause = match limit {
        Some(n) if n > 0 => format!("LIMIT {n}"),
        _ => String::new(),
    };
    format!(
        "SELECT p.id AS product_id, p.name AS name, c.name AS category,
                SUM(si.qty - si.refunded_qty) AS qty,
                TOTAL((si.price - si.discount) * {NET_ITEM_QTY}) AS revenue,
                TOTAL((si.price - si.discount - si.cost_price) * {NET_ITEM_QTY}) AS profit
         FROM sale_items si
         JOIN sales s ON s.id = si.sale_id
         JOIN products p ON p.id = si.product_id
         LEFT JOIN categories c ON c.id = p.category_id
         WHERE {SALES_PERIOD_FILTER}
         GROUP BY p.id HAVING SUM(si.qty - si.refunded_qty) > 0 ORDER BY revenue DESC {limit_clause}"
    )
}

pub(crate) async fn compute_top_products(
    pool: &sqlx::SqlitePool,
    from: &str,
    to: &str,
    limit: Option<i64>,
) -> Result<Vec<TopProductRow>, String> {
    let rows: Vec<(i64, String, Option<String>, f64, f64, f64)> =
        sqlx::query_as(&top_products_sql(limit))
            .bind(from)
            .bind(to)
            .fetch_all(pool)
            .await
            .map_err(|e| e.to_string())?;
    Ok(rows
        .into_iter()
        .map(
            |(product_id, name, category, qty, revenue, profit)| TopProductRow {
                product_id,
                name,
                category,
                qty,
                revenue,
                profit,
            },
        )
        .collect())
}

#[tauri::command]
pub async fn top_products<R: Runtime>(
    app: AppHandle<R>,
    from: String,
    to: String,
    limit: Option<i64>,
) -> Result<Vec<TopProductRow>, String> {
    validate_range(&from, &to)?;
    let pool = db::pool(&app).await?;
    compute_top_products(&pool, &from, &to, limit.or(Some(10))).await
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategorySalesRow {
    pub category: String,
    pub qty: f64,
    pub revenue: f64,
    pub profit: f64,
}

pub(crate) async fn compute_category_breakdown(
    pool: &sqlx::SqlitePool,
    from: &str,
    to: &str,
) -> Result<Vec<CategorySalesRow>, String> {
    let rows: Vec<(Option<String>, f64, f64, f64)> = sqlx::query_as(&format!(
        "SELECT COALESCE(c.name, '—') AS category,
                SUM(si.qty - si.refunded_qty) AS qty,
                TOTAL((si.price - si.discount) * {NET_ITEM_QTY}) AS revenue,
                TOTAL((si.price - si.discount - si.cost_price) * {NET_ITEM_QTY}) AS profit
         FROM sale_items si
         JOIN sales s ON s.id = si.sale_id
         JOIN products p ON p.id = si.product_id
         LEFT JOIN categories c ON c.id = p.category_id
         WHERE {SALES_PERIOD_FILTER}
         GROUP BY c.name HAVING SUM(si.qty - si.refunded_qty) > 0
         ORDER BY revenue DESC"
    ))
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(rows
        .into_iter()
        .map(|(category, qty, revenue, profit)| CategorySalesRow {
            category: category.unwrap_or_else(|| "—".into()),
            qty,
            revenue,
            profit,
        })
        .collect())
}

#[tauri::command]
pub async fn category_breakdown<R: Runtime>(
    app: AppHandle<R>,
    from: String,
    to: String,
) -> Result<Vec<CategorySalesRow>, String> {
    validate_range(&from, &to)?;
    let pool = db::pool(&app).await?;
    compute_category_breakdown(&pool, &from, &to).await
}

// ============================================================
// F6.3 Sales report
// ============================================================

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SalesReportRow {
    pub id: i64,
    pub sale_no: String,
    pub created_at: String,
    pub cashier: String,
    pub customer: Option<String>,
    pub subtotal: f64,
    pub discount: f64,
    pub total: f64,
    /// Amount already refunded against this invoice.
    pub refunded: f64,
    pub status: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SalesReportOutput {
    pub rows: Vec<SalesReportRow>,
    pub orders: i64,
    pub revenue: f64,
    pub avg_ticket: f64,
}

pub(crate) struct SalesFilter<'a> {
    from: &'a str,
    to: &'a str,
    cashier_id: Option<i64>,
    customer_id: Option<i64>,
    include_voided: bool,
}

impl<'a> SalesFilter<'a> {
    /// Builds the WHERE clause using SQLite numbered placeholders. Placeholders
    /// are numbered in bind order: ?1 = from, ?2 = to, ?3 = cashier, ?4 = customer
    /// (filters only present when set).
    fn where_sql(&self) -> String {
        let mut sql = String::from("date(s.created_at, 'localtime') BETWEEN ?1 AND ?2");
        if !self.include_voided {
            sql.push_str(" AND s.status = 'completed'");
        }
        let mut next = 3;
        if self.cashier_id.is_some() {
            sql.push_str(&format!(" AND s.user_id = ?{next}"));
            next += 1;
        }
        if self.customer_id.is_some() {
            sql.push_str(&format!(" AND s.customer_id = ?{next}"));
        }
        sql
    }
}

pub(crate) async fn compute_sales_report(
    pool: &sqlx::SqlitePool,
    filter: &SalesFilter<'_>,
) -> Result<SalesReportOutput, String> {
    let where_sql = filter.where_sql();

    let sql = format!(
        "SELECT s.id, s.sale_no, s.created_at, u.full_name AS cashier, c.name AS customer,
                s.subtotal, s.discount, s.total,
                COALESCE((SELECT SUM(r.amount) FROM refunds r WHERE r.sale_id = s.id), 0.0) AS refunded,
                s.status
         FROM sales s
         JOIN users u ON u.id = s.user_id
         LEFT JOIN customers c ON c.id = s.customer_id
         WHERE {where_sql}
         ORDER BY s.created_at DESC, s.id DESC"
    );
    let totals_sql =
        format!("SELECT COUNT(*), TOTAL({NET_SALE_TOTAL}) FROM sales s WHERE {where_sql}");

    let mut query = sqlx::query_as::<_, (i64, String, String, String, Option<String>, f64, f64, f64, f64, String)>(
        &sql,
    )
    .bind(filter.from)
    .bind(filter.to);
    let mut totals_query = sqlx::query_as::<_, (i64, f64)>(&totals_sql)
        .bind(filter.from)
        .bind(filter.to);
    if let Some(cid) = filter.cashier_id {
        query = query.bind(cid);
        totals_query = totals_query.bind(cid);
    }
    if let Some(kid) = filter.customer_id {
        query = query.bind(kid);
        totals_query = totals_query.bind(kid);
    }

    let rows = query.fetch_all(pool).await.map_err(|e| e.to_string())?;
    let totals = totals_query.fetch_one(pool).await.map_err(|e| e.to_string())?;

    let rows = rows
        .into_iter()
        .map(
            |(id, sale_no, created_at, cashier, customer, subtotal, discount, total, refunded, status)| {
                SalesReportRow {
                    id,
                    sale_no,
                    created_at,
                    cashier,
                    customer,
                    subtotal,
                    discount,
                    total,
                    refunded,
                    status,
                }
            },
        )
        .collect();

    let orders = totals.0;
    let revenue = totals.1;
    Ok(SalesReportOutput {
        rows,
        orders,
        revenue,
        avg_ticket: if orders > 0 { revenue / orders as f64 } else { 0.0 },
    })
}

#[tauri::command]
pub async fn sales_report<R: Runtime>(
    app: AppHandle<R>,
    from: String,
    to: String,
    cashier_id: Option<i64>,
    customer_id: Option<i64>,
    include_voided: Option<bool>,
) -> Result<SalesReportOutput, String> {
    validate_range(&from, &to)?;
    let pool = db::pool(&app).await?;
    compute_sales_report(
        &pool,
        &SalesFilter {
            from: &from,
            to: &to,
            cashier_id,
            customer_id,
            include_voided: include_voided.unwrap_or(false),
        },
    )
    .await
}

// ============================================================
// F6.6 Inventory report
// ============================================================

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryRow {
    pub id: i64,
    pub name: String,
    pub sku: Option<String>,
    pub category: Option<String>,
    pub stock_qty: f64,
    pub reorder_level: f64,
    pub cost_price: f64,
    pub sell_price: f64,
    pub stock_value: f64,
    pub low_stock: bool,
}

pub(crate) async fn compute_inventory(pool: &sqlx::SqlitePool) -> Result<Vec<InventoryRow>, String> {
    let rows: Vec<(i64, String, Option<String>, Option<String>, f64, f64, f64, f64)> =
        sqlx::query_as(
            "SELECT p.id, p.name, p.sku, c.name, p.stock_qty, p.reorder_level,
                    p.cost_price, p.sell_price
             FROM products p
             LEFT JOIN categories c ON c.id = p.category_id
             WHERE p.is_active = 1
             ORDER BY p.name COLLATE NOCASE",
        )
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(rows
        .into_iter()
        .map(
            |(id, name, sku, category, stock_qty, reorder_level, cost_price, sell_price)| {
                InventoryRow {
                    id,
                    stock_value: stock_qty * cost_price,
                    low_stock: stock_qty <= reorder_level,
                    name,
                    sku,
                    category,
                    stock_qty,
                    reorder_level,
                    cost_price,
                    sell_price,
                }
            },
        )
        .collect())
}

#[tauri::command]
pub async fn inventory_report<R: Runtime>(app: AppHandle<R>) -> Result<Vec<InventoryRow>, String> {
    let pool = db::pool(&app).await?;
    compute_inventory(&pool).await
}

// ============================================================
// F6.7 Profit margins
// ============================================================

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarginRow {
    pub id: i64,
    pub name: String,
    pub category: Option<String>,
    pub qty_sold: f64,
    pub revenue: f64,
    pub cogs: f64,
    pub profit: f64,
    pub margin_pct: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarginReport {
    pub products: Vec<MarginRow>,
    pub categories: Vec<MarginRow>,
}

const MARGIN_EXPR: &str =
    "TOTAL((si.price - si.discount) * (si.qty - si.refunded_qty)) AS revenue,
     TOTAL(si.cost_price * (si.qty - si.refunded_qty)) AS cogs,
     TOTAL((si.price - si.discount - si.cost_price) * (si.qty - si.refunded_qty)) AS profit";

fn pct(profit: f64, revenue: f64) -> f64 {
    if revenue.abs() < f64::EPSILON {
        0.0
    } else {
        profit / revenue * 100.0
    }
}

pub(crate) async fn compute_margins(
    pool: &sqlx::SqlitePool,
    from: &str,
    to: &str,
) -> Result<MarginReport, String> {
    let product_rows: Vec<(i64, String, Option<String>, f64, f64, f64, f64)> = sqlx::query_as(&format!(
        "SELECT p.id, p.name, c.name, SUM(si.qty - si.refunded_qty) AS qty_sold, {MARGIN_EXPR}
         FROM sale_items si
         JOIN sales s ON s.id = si.sale_id
         JOIN products p ON p.id = si.product_id
         LEFT JOIN categories c ON c.id = p.category_id
         WHERE {SALES_PERIOD_FILTER}
         GROUP BY p.id HAVING SUM(si.qty - si.refunded_qty) > 0
         ORDER BY profit DESC"
    ))
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let category_rows: Vec<(String, f64, f64, f64, f64)> = sqlx::query_as(&format!(
        "SELECT COALESCE(c.name, '—') AS name, SUM(si.qty - si.refunded_qty) AS qty_sold, {MARGIN_EXPR}
         FROM sale_items si
         JOIN sales s ON s.id = si.sale_id
         JOIN products p ON p.id = si.product_id
         LEFT JOIN categories c ON c.id = p.category_id
         WHERE {SALES_PERIOD_FILTER}
         GROUP BY c.name HAVING SUM(si.qty - si.refunded_qty) > 0
         ORDER BY profit DESC"
    ))
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let map_product = |(id, name, category, qty_sold, revenue, cogs, profit): (
        i64,
        String,
        Option<String>,
        f64,
        f64,
        f64,
        f64,
    )| {
        MarginRow {
            id,
            margin_pct: pct(profit, revenue),
            name,
            category,
            qty_sold,
            revenue,
            cogs,
            profit,
        }
    };

    let products = product_rows.into_iter().map(map_product).collect();
    let categories = category_rows
        .into_iter()
        .map(|(name, qty_sold, revenue, cogs, profit)| MarginRow {
            id: 0,
            margin_pct: pct(profit, revenue),
            name,
            category: None,
            qty_sold,
            revenue,
            cogs,
            profit,
        })
        .collect();

    Ok(MarginReport {
        products,
        categories,
    })
}

#[tauri::command]
pub async fn margin_report<R: Runtime>(
    app: AppHandle<R>,
    from: String,
    to: String,
) -> Result<MarginReport, String> {
    validate_range(&from, &to)?;
    let pool = db::pool(&app).await?;
    compute_margins(&pool, &from, &to).await
}

// ============================================================
// F6.8 Exports (.xlsx)
// ============================================================

fn money(v: f64) -> String {
    crate::format::money(v)
}

fn num(v: f64) -> String {
    if v.fract().abs() < f64::EPSILON {
        format!("{}", v as i64)
    } else {
        format!("{v:.3}")
    }
}

#[tauri::command]
pub async fn export_sales_report<R: Runtime>(
    app: AppHandle<R>,
    path: String,
    from: String,
    to: String,
    cashier_id: Option<i64>,
    customer_id: Option<i64>,
    include_voided: Option<bool>,
) -> Result<(), String> {
    validate_range(&from, &to)?;
    let pool = db::pool(&app).await?;
    let report = compute_sales_report(
        &pool,
        &SalesFilter {
            from: &from,
            to: &to,
            cashier_id,
            customer_id,
            include_voided: include_voided.unwrap_or(false),
        },
    )
    .await?;

    let headers = ["Sale No", "Date", "Cashier", "Customer", "Subtotal", "Discount", "Total", "Refunded", "Net", "Status"];
    let rows: Vec<Vec<String>> = report
        .rows
        .iter()
        .map(|r| {
            vec![
                r.sale_no.clone(),
                r.created_at.clone(),
                r.cashier.clone(),
                r.customer.clone().unwrap_or_default(),
                money(r.subtotal),
                money(r.discount),
                money(r.total),
                money(r.refunded),
                money(r.total - r.refunded),
                r.status.clone(),
            ]
        })
        .collect();
    crate::export::write_xlsx(std::path::Path::new(&path), "Sales", &headers, &rows)
}

#[tauri::command]
pub async fn export_inventory<R: Runtime>(app: AppHandle<R>, path: String) -> Result<(), String> {
    let pool = db::pool(&app).await?;
    let items = compute_inventory(&pool).await?;
    let headers = [
        "SKU",
        "Name",
        "Category",
        "Stock",
        "Reorder level",
        "Cost price",
        "Sell price",
        "Stock value",
        "Low stock",
    ];
    let rows: Vec<Vec<String>> = items
        .iter()
        .map(|i| {
            vec![
                i.sku.clone().unwrap_or_default(),
                i.name.clone(),
                i.category.clone().unwrap_or_default(),
                num(i.stock_qty),
                num(i.reorder_level),
                money(i.cost_price),
                money(i.sell_price),
                money(i.stock_value),
                if i.low_stock { "YES" } else { "" }.to_string(),
            ]
        })
        .collect();
    crate::export::write_xlsx(std::path::Path::new(&path), "Inventory", &headers, &rows)
}

#[tauri::command]
pub async fn export_top_products<R: Runtime>(
    app: AppHandle<R>,
    path: String,
    from: String,
    to: String,
) -> Result<(), String> {
    validate_range(&from, &to)?;
    let pool = db::pool(&app).await?;
    let items = compute_top_products(&pool, &from, &to, None).await?;
    let headers = ["Name", "Category", "Qty sold", "Revenue", "Profit"];
    let rows: Vec<Vec<String>> = items
        .iter()
        .map(|p| {
            vec![
                p.name.clone(),
                p.category.clone().unwrap_or_default(),
                num(p.qty),
                money(p.revenue),
                money(p.profit),
            ]
        })
        .collect();
    crate::export::write_xlsx(std::path::Path::new(&path), "Top Products", &headers, &rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schema() -> &'static str {
        r#"
        CREATE TABLE categories (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          name TEXT UNIQUE NOT NULL
        );
        CREATE TABLE products (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          name TEXT NOT NULL,
          sku TEXT,
          category_id INTEGER REFERENCES categories(id),
          cost_price REAL NOT NULL DEFAULT 0,
          sell_price REAL NOT NULL DEFAULT 0,
          stock_qty REAL NOT NULL DEFAULT 0,
          reorder_level REAL NOT NULL DEFAULT 0,
          is_active INTEGER NOT NULL DEFAULT 1
        );
        CREATE TABLE users (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          username TEXT NOT NULL,
          full_name TEXT
        );
        CREATE TABLE customers (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          name TEXT NOT NULL,
          balance REAL NOT NULL DEFAULT 0
        );
        CREATE TABLE sales (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          sale_no TEXT UNIQUE NOT NULL,
          customer_id INTEGER,
          user_id INTEGER NOT NULL,
          subtotal REAL NOT NULL DEFAULT 0,
          discount REAL NOT NULL DEFAULT 0,
          tax REAL NOT NULL DEFAULT 0,
          total REAL NOT NULL DEFAULT 0,
          status TEXT NOT NULL DEFAULT 'completed',
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
          subtotal REAL NOT NULL,
          refunded_qty REAL NOT NULL DEFAULT 0
        );
        CREATE TABLE refunds (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          sale_no TEXT UNIQUE NOT NULL,
          sale_id INTEGER NOT NULL REFERENCES sales(id),
          session_id INTEGER,
          user_id INTEGER,
          customer_id INTEGER,
          method TEXT NOT NULL,
          reason TEXT,
          amount REAL NOT NULL,
          created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE TABLE expense_out (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          amount REAL NOT NULL,
          date TEXT NOT NULL DEFAULT (datetime('now'))
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

        sqlx::query("INSERT INTO users (id, username, full_name) VALUES (1, 'admin', 'Admin User')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO customers (id, name) VALUES (1, 'Omar')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO categories (id, name) VALUES (1, 'Drinks')")
            .execute(&pool)
            .await
            .unwrap();
        // Product 1: buy 2, sell 5 → profit 3/unit sold.
        sqlx::query(
            "INSERT INTO products (id, name, category_id, cost_price, sell_price, stock_qty, reorder_level)
             VALUES (1, 'Coffee', 1, 2.0, 5.0, 4.0, 5.0)",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO products (id, name, category_id, cost_price, sell_price, stock_qty, reorder_level)
             VALUES (2, 'Tea', 1, 1.0, 3.0, 9.0, 5.0)",
        )
        .execute(&pool)
        .await
        .unwrap();

        // Sale 1 (today): 2× Coffee @5, cost 2 → revenue 10, profit 6.
        sqlx::query(
            "INSERT INTO sales (id, sale_no, user_id, customer_id, subtotal, discount, total, status, created_at)
             VALUES (1, 'S-000001', 1, 1, 10, 0, 10, 'completed', datetime('now', 'localtime'))",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO sale_items (sale_id, product_id, qty, price, cost_price, discount, subtotal)
             VALUES (1, 1, 2, 5.0, 2.0, 0, 10.0)",
        )
        .execute(&pool)
        .await
        .unwrap();

        // Sale 2 (today): 1× Tea @3 → revenue 3, profit 2.
        sqlx::query(
            "INSERT INTO sales (id, sale_no, user_id, subtotal, discount, total, status, created_at)
             VALUES (2, 'S-000002', 1, 3, 0, 3, 'completed', datetime('now', 'localtime'))",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO sale_items (sale_id, product_id, qty, price, cost_price, discount, subtotal)
             VALUES (2, 2, 1, 3.0, 1.0, 0, 3.0)",
        )
        .execute(&pool)
        .await
        .unwrap();

        // Voided sale must be ignored everywhere.
        sqlx::query(
            "INSERT INTO sales (id, sale_no, user_id, subtotal, discount, total, status, created_at)
             VALUES (3, 'S-000003', 1, 99, 0, 99, 'voided', datetime('now', 'localtime'))",
        )
        .execute(&pool)
        .await
        .unwrap();

        // Expense of 4 today.
        sqlx::query("INSERT INTO expense_out (amount, date) VALUES (4.0, datetime('now', 'localtime'))")
            .execute(&pool)
            .await
            .unwrap();

        pool
    }

    fn today() -> (String, String) {
        // Use a wide range so local/UTC date shifts cannot break assertions.
        ("2000-01-01".into(), "2999-12-31".into())
    }

    #[tokio::test]
    async fn summary_aggregates_revenue_profit_and_expenses() {
        let pool = mem_pool().await;
        let (from, to) = today();
        let s = compute_summary(&pool, &from, &to).await.unwrap();

        assert_eq!(s.orders, 2);
        assert!((s.revenue - 13.0).abs() < 0.001);
        assert!((s.avg_ticket - 6.5).abs() < 0.001);
        assert!((s.gross_profit - 8.0).abs() < 0.001); // 6 + 2
        assert!((s.expenses_total - 4.0).abs() < 0.001);
        assert!((s.net_position - 9.0).abs() < 0.001); // revenue − expenses
    }

    #[tokio::test]
    async fn empty_period_decodes_as_zero_real() {
        // Regression: COALESCE(SUM(...), 0) used to yield an INTEGER that sqlx
        // refused to decode into f64 on periods without any sales.
        let pool = mem_pool().await;

        let s = compute_summary(&pool, "1999-01-01", "1999-01-02").await.unwrap();
        assert_eq!(s.orders, 0);
        assert!(s.revenue.abs() < 0.001);
        assert!(s.gross_profit.abs() < 0.001);

        let trend = compute_trend(&pool, "1999-01-01", "1999-01-02", "day")
            .await
            .unwrap();
        assert!(trend.is_empty());

        let report = compute_sales_report(
            &pool,
            &SalesFilter {
                from: "1999-01-01",
                to: "1999-01-02",
                cashier_id: None,
                customer_id: None,
                include_voided: false,
            },
        )
        .await
        .unwrap();
        assert_eq!(report.orders, 0);
        assert!(report.revenue.abs() < 0.001);
    }

    #[tokio::test]
    async fn trend_buckets_by_day() {
        let pool = mem_pool().await;
        let (from, to) = today();
        let points = compute_trend(&pool, &from, &to, "day").await.unwrap();

        assert_eq!(points.len(), 1);
        assert!((points[0].revenue - 13.0).abs() < 0.001);
        assert_eq!(points[0].orders, 2);
        assert!(compute_trend(&pool, &from, &to, "bogus").await.is_err());
    }

    #[tokio::test]
    async fn top_products_rank_by_revenue() {
        let pool = mem_pool().await;
        let (from, to) = today();
        let top = compute_top_products(&pool, &from, &to, Some(1)).await.unwrap();

        assert_eq!(top.len(), 1);
        assert_eq!(top[0].name, "Coffee");
        assert!((top[0].qty - 2.0).abs() < 0.001);
        assert!((top[0].profit - 6.0).abs() < 0.001);

        let cats = compute_category_breakdown(&pool, &from, &to).await.unwrap();
        assert_eq!(cats.len(), 1);
        assert_eq!(cats[0].category, "Drinks");
        assert!((cats[0].revenue - 13.0).abs() < 0.001);
    }

    #[tokio::test]
    async fn sales_report_filters_by_cashier_and_customer() {
        let pool = mem_pool().await;
        let (from, to) = today();

        let all = compute_sales_report(
            &pool,
            &SalesFilter {
                from: &from,
                to: &to,
                cashier_id: None,
                customer_id: None,
                include_voided: false,
            },
        )
        .await
        .unwrap();
        assert_eq!(all.rows.len(), 2);
        assert_eq!(all.orders, 2);
        assert!((all.revenue - 13.0).abs() < 0.001);

        let by_customer = compute_sales_report(
            &pool,
            &SalesFilter {
                from: &from,
                to: &to,
                cashier_id: None,
                customer_id: Some(1),
                include_voided: false,
            },
        )
        .await
        .unwrap();
        assert_eq!(by_customer.rows.len(), 1);
        assert_eq!(by_customer.rows[0].customer.as_deref(), Some("Omar"));

        let voided = compute_sales_report(
            &pool,
            &SalesFilter {
                from: &from,
                to: &to,
                cashier_id: None,
                customer_id: None,
                include_voided: true,
            },
        )
        .await
        .unwrap();
        assert_eq!(voided.rows.len(), 3);
    }

    #[tokio::test]
    async fn inventory_flags_low_stock_and_values() {
        let pool = mem_pool().await;
        let items = compute_inventory(&pool).await.unwrap();

        let coffee = items.iter().find(|i| i.name == "Coffee").unwrap();
        assert!(coffee.low_stock); // 4 ≤ 5
        assert!((coffee.stock_value - 8.0).abs() < 0.001);

        let tea = items.iter().find(|i| i.name == "Tea").unwrap();
        assert!(!tea.low_stock); // 9 > 5
    }

    #[tokio::test]
    async fn margins_compute_percentages() {
        let pool = mem_pool().await;
        let (from, to) = today();
        let m = compute_margins(&pool, &from, &to).await.unwrap();

        let coffee = m.products.iter().find(|p| p.name == "Coffee").unwrap();
        assert!((coffee.profit - 6.0).abs() < 0.001);
        assert!((coffee.margin_pct - 60.0).abs() < 0.01);

        assert_eq!(m.categories.len(), 1);
        assert!((m.categories[0].cogs - 5.0).abs() < 0.001);
    }

    /// Partial refund of one coffee on sale 1 (10 → 5 net) must shrink every
    /// revenue/qty/profit aggregate accordingly.
    #[tokio::test]
    async fn refunds_reduce_all_report_aggregates() {
        let pool = mem_pool().await;
        let (from, to) = today();

        // Refund one of the two coffees: money back 5, unit returned.
        sqlx::query(
            "INSERT INTO refunds (sale_no, sale_id, method, amount) VALUES ('R-000001', 1, 'cash', 5.0)",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("UPDATE sale_items SET refunded_qty = 1 WHERE sale_id = 1 AND product_id = 1")
            .execute(&pool)
            .await
            .unwrap();

        let s = compute_summary(&pool, &from, &to).await.unwrap();
        assert_eq!(s.orders, 2); // invoice count unchanged
        assert!((s.revenue - 8.0).abs() < 0.001); // 10 − 5 + 3
        assert!((s.avg_ticket - 4.0).abs() < 0.001);
        assert!((s.gross_profit - 5.0).abs() < 0.001); // 3 + 2
        assert!((s.net_position - 4.0).abs() < 0.001); // 8 − 4

        let points = compute_trend(&pool, &from, &to, "day").await.unwrap();
        assert_eq!(points.len(), 1);
        assert!((points[0].revenue - 8.0).abs() < 0.001);

        let top = compute_top_products(&pool, &from, &to, None).await.unwrap();
        let coffee = top.iter().find(|p| p.name == "Coffee").unwrap();
        assert!((coffee.qty - 1.0).abs() < 0.001);
        assert!((coffee.revenue - 5.0).abs() < 0.001);
        assert!((coffee.profit - 3.0).abs() < 0.001);

        let cats = compute_category_breakdown(&pool, &from, &to).await.unwrap();
        assert_eq!(cats.len(), 1);
        assert!((cats[0].revenue - 8.0).abs() < 0.001);

        let m = compute_margins(&pool, &from, &to).await.unwrap();
        let coffee = m.products.iter().find(|p| p.name == "Coffee").unwrap();
        assert!((coffee.qty_sold - 1.0).abs() < 0.001);
        assert!((coffee.cogs - 2.0).abs() < 0.001);
        assert!((coffee.profit - 3.0).abs() < 0.001);

        let report = compute_sales_report(
            &pool,
            &SalesFilter {
                from: &from,
                to: &to,
                cashier_id: None,
                customer_id: None,
                include_voided: false,
            },
        )
        .await
        .unwrap();
        assert!((report.revenue - 8.0).abs() < 0.001);
        let row1 = report.rows.iter().find(|r| r.sale_no == "S-000001").unwrap();
        assert!((row1.refunded - 5.0).abs() < 0.001);
    }

    /// A fully refunded line drops out of item-level aggregates entirely.
    #[tokio::test]
    async fn fully_refunded_line_disappears_from_item_reports() {
        let pool = mem_pool().await;
        let (from, to) = today();

        sqlx::query(
            "INSERT INTO refunds (sale_no, sale_id, method, amount) VALUES ('R-000002', 1, 'cash', 10.0)",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("UPDATE sale_items SET refunded_qty = qty WHERE sale_id = 1 AND product_id = 1")
            .execute(&pool)
            .await
            .unwrap();

        let top = compute_top_products(&pool, &from, &to, None).await.unwrap();
        assert!(top.iter().all(|p| p.name != "Coffee"));

        let s = compute_summary(&pool, &from, &to).await.unwrap();
        assert!((s.revenue - 3.0).abs() < 0.001); // only the tea remains
    }

    /// Trend buckets whose invoices are all fully refunded vanish so the UI
    /// shows the empty state instead of an all-zero chart; partial refunds
    /// keep the bucket at the reduced net revenue.
    #[tokio::test]
    async fn trend_drops_fully_refunded_buckets_keeps_partial() {
        // Everything refunded: no trend points at all.
        let pool = mem_pool().await;
        let (from, to) = today();
        sqlx::query(
            "INSERT INTO refunds (sale_no, sale_id, method, amount) VALUES ('R-000005', 1, 'cash', 10.0)",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO refunds (sale_no, sale_id, method, amount) VALUES ('R-000006', 2, 'cash', 3.0)",
        )
        .execute(&pool)
        .await
        .unwrap();
        let points = compute_trend(&pool, &from, &to, "day").await.unwrap();
        assert!(points.is_empty());

        // Partial refund: bucket stays with the reduced revenue.
        let pool2 = mem_pool().await;
        sqlx::query(
            "INSERT INTO refunds (sale_no, sale_id, method, amount) VALUES ('R-000007', 1, 'cash', 5.0)",
        )
        .execute(&pool2)
        .await
        .unwrap();
        let points = compute_trend(&pool2, &from, &to, "day").await.unwrap();
        assert_eq!(points.len(), 1);
        assert!((points[0].revenue - 8.0).abs() < 0.001);
        assert_eq!(points[0].orders, 2);
    }

    /// Regression: `sales.created_at` is stored in UTC (`datetime('now')`) while
    /// the report range is expressed in local dates. A sale late in the UTC day
    /// must appear under its *local* date, not the raw UTC date.
    #[tokio::test]
    async fn sales_are_bucketed_by_local_date_not_utc_date() {
        let pool = mem_pool().await;

        sqlx::query("DELETE FROM sale_items").execute(&pool).await.unwrap();
        sqlx::query("DELETE FROM refunds").execute(&pool).await.unwrap();
        sqlx::query("DELETE FROM expense_out").execute(&pool).await.unwrap();
        sqlx::query("DELETE FROM sales").execute(&pool).await.unwrap();

        sqlx::query(
            "INSERT INTO sales (id, sale_no, user_id, subtotal, discount, total, status, created_at)
             VALUES (1, 'S-TZ001', 1, 20, 0, 20, 'completed', '2026-08-28 22:00:00')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO sale_items (sale_id, product_id, qty, price, cost_price, discount, subtotal)
             VALUES (1, 1, 1, 20.0, 2.0, 0, 20.0)",
        )
        .execute(&pool)
        .await
        .unwrap();

        let (utc_day, local_day): (String, String) = sqlx::query_as(
            "SELECT date('2026-08-28 22:00:00'), date('2026-08-28 22:00:00', 'localtime')",
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        let by_local = compute_summary(&pool, &local_day, &local_day).await.unwrap();
        assert_eq!(by_local.orders, 1);
        assert!((by_local.revenue - 20.0).abs() < 0.001);

        // When the store's local date differs from the UTC date, the raw UTC
        // date must NOT match — this is the bug the fix addresses.
        if utc_day != local_day {
            let by_utc = compute_summary(&pool, &utc_day, &utc_day).await.unwrap();
            assert_eq!(by_utc.orders, 0);
        }
    }
}
