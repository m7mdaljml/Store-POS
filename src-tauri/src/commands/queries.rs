use serde::Serialize;
use sqlx::Row;
use tauri::{AppHandle, Runtime};

use super::guard;
use crate::db;

/// A product row for the offline checkout catalog. Mirrors the columns the
/// frontend previously selected directly (including the resolved tax rate).
#[derive(Debug, Serialize)]
pub struct CatalogProduct {
    pub id: i64,
    pub sku: Option<String>,
    pub barcode: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub category_id: Option<i64>,
    pub cost_price: f64,
    pub sell_price: f64,
    pub tax_profile_id: Option<i64>,
    pub tax_rate: f64,
    pub unit: String,
    pub stock_qty: f64,
    pub reorder_level: f64,
    pub image_path: Option<String>,
    pub is_active: i64,
    pub is_quick: i64,
}

/// Full active/inactive product catalog used by checkout, products and the
/// dashboard. Replaces the direct `SELECT ... FROM products` in the webview.
#[tauri::command]
pub async fn list_products<R: Runtime>(app: AppHandle<R>) -> Result<Vec<CatalogProduct>, String> {
    guard::require(&app, None)?;
    let pool = db::pool(&app).await?;
    let rows = sqlx::query(
        "SELECT p.id, p.sku, p.barcode, p.name, p.description, p.category_id,
                p.cost_price, p.sell_price, p.tax_profile_id,
                COALESCE(tp.rate, 0.0) AS tax_rate, p.unit, p.stock_qty,
                p.reorder_level, p.image_path, p.is_active, p.is_quick
         FROM products p
         LEFT JOIN tax_profiles tp ON tp.id = p.tax_profile_id",
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| e.to_string())?;

    rows.into_iter()
        .map(|row| {
            Ok(CatalogProduct {
                id: row.try_get("id").map_err(|e| e.to_string())?,
                sku: row.try_get("sku").map_err(|e| e.to_string())?,
                barcode: row.try_get("barcode").map_err(|e| e.to_string())?,
                name: row.try_get("name").map_err(|e| e.to_string())?,
                description: row.try_get("description").map_err(|e| e.to_string())?,
                category_id: row.try_get("category_id").map_err(|e| e.to_string())?,
                cost_price: row.try_get("cost_price").map_err(|e| e.to_string())?,
                sell_price: row.try_get("sell_price").map_err(|e| e.to_string())?,
                tax_profile_id: row.try_get("tax_profile_id").map_err(|e| e.to_string())?,
                tax_rate: row.try_get("tax_rate").map_err(|e| e.to_string())?,
                unit: row.try_get("unit").map_err(|e| e.to_string())?,
                stock_qty: row.try_get("stock_qty").map_err(|e| e.to_string())?,
                reorder_level: row.try_get("reorder_level").map_err(|e| e.to_string())?,
                image_path: row.try_get("image_path").map_err(|e| e.to_string())?,
                is_active: row.try_get("is_active").map_err(|e| e.to_string())?,
                is_quick: row.try_get("is_quick").map_err(|e| e.to_string())?,
            })
        })
        .collect()
}

#[derive(Debug, Serialize)]
pub struct BaseCurrency {
    pub code: String,
    pub symbol: String,
}

/// Base currency code/symbol for money formatting, with the same settings
/// fallback the frontend used when no `is_base` row exists.
#[tauri::command]
pub async fn get_base_currency<R: Runtime>(app: AppHandle<R>) -> Result<BaseCurrency, String> {
    guard::require(&app, None)?;
    let pool = db::pool(&app).await?;

    if let Some(row) = sqlx::query("SELECT code, symbol FROM currencies WHERE is_base = 1 LIMIT 1")
        .fetch_optional(&pool)
        .await
        .map_err(|e| e.to_string())?
    {
        let code: String = row.try_get("code").map_err(|e| e.to_string())?;
        let symbol: Option<String> = row.try_get("symbol").map_err(|e| e.to_string())?;
        let symbol = symbol.unwrap_or_default().trim().to_string();
        if !symbol.is_empty() || !code.trim().is_empty() {
            return Ok(BaseCurrency {
                symbol: if symbol.is_empty() { code.trim().to_string() } else { symbol },
                code: code.trim().to_string(),
            });
        }
    }

    let fallback: Option<(String,)> =
        sqlx::query_as("SELECT value FROM settings WHERE key = 'currency' LIMIT 1")
            .fetch_optional(&pool)
            .await
            .map_err(|e| e.to_string())?;
    let value = fallback.map(|(v,)| v.trim().to_string()).unwrap_or_default();
    Ok(BaseCurrency {
        code: value.clone(),
        symbol: value,
    })
}

/// Timestamp of the most recent backup (UTC), or `None` when none exist.
#[tauri::command]
pub async fn last_backup_time<R: Runtime>(app: AppHandle<R>) -> Result<Option<String>, String> {
    guard::require(&app, None)?;
    let pool = db::pool(&app).await?;
    let row: Option<(Option<String>,)> = sqlx::query_as("SELECT MAX(created_at) FROM backups")
        .fetch_optional(&pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(row.and_then(|(v,)| v))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardKpis {
    pub revenue: f64,
    pub orders: i64,
    pub avg_ticket: f64,
    pub month_revenue: f64,
    pub today_taxes: f64,
}

/// Today's and this month's sales KPIs for the dashboard. Admin-only, matching
/// the `adminOnly` guard on the dashboard route.
#[tauri::command]
pub async fn dashboard_kpis<R: Runtime>(app: AppHandle<R>) -> Result<DashboardKpis, String> {
    guard::require_admin(&app)?;
    let pool = db::pool(&app).await?;

    let today = sqlx::query(
        "SELECT
            TOTAL(total - COALESCE((SELECT TOTAL(amount) FROM refunds WHERE sale_id = sales.id), 0.0)) AS revenue,
            COUNT(*) AS orders,
            TOTAL(tax - COALESCE(tax * (SELECT TOTAL(amount) FROM refunds WHERE sale_id = sales.id) / NULLIF(total, 0.0), 0.0)) AS taxes
         FROM sales
         WHERE status = 'completed' AND date(created_at, 'localtime') = date('now', 'localtime')",
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| e.to_string())?;

    let month = sqlx::query(
        "SELECT TOTAL(total - COALESCE((SELECT TOTAL(amount) FROM refunds WHERE sale_id = sales.id), 0.0)) AS revenue
         FROM sales
         WHERE status = 'completed'
           AND strftime('%Y-%m', created_at, 'localtime') = strftime('%Y-%m', 'now', 'localtime')",
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| e.to_string())?;

    let revenue: f64 = today.try_get("revenue").map_err(|e| e.to_string())?;
    let orders: i64 = today.try_get("orders").map_err(|e| e.to_string())?;
    let taxes: f64 = today.try_get("taxes").map_err(|e| e.to_string())?;
    let month_revenue: f64 = month.try_get("revenue").map_err(|e| e.to_string())?;

    Ok(DashboardKpis {
        revenue,
        orders,
        avg_ticket: if orders > 0 { revenue / orders as f64 } else { 0.0 },
        month_revenue,
        today_taxes: taxes,
    })
}
