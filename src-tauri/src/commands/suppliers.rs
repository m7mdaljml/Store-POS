use serde::{Deserialize, Serialize};
use sqlx::Row;
use tauri::{AppHandle, Runtime};

use super::Page;
use crate::db;

#[derive(Debug, Serialize)]
pub struct SupplierRecord {
    pub id: i64,
    pub name: String,
    pub contact: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub tax_id: Option<String>,
    pub created_at: String,
    pub invoice_count: i64,
    pub total_purchased: f64,
    pub total_due: f64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SupplierInput {
    pub name: String,
    pub contact: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub tax_id: Option<String>,
}

fn validate_supplier(input: &SupplierInput) -> Result<(), String> {
    if input.name.trim().is_empty() {
        return Err("Supplier name is required".into());
    }
    if let Some(email) = input.email.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
        if !email.contains('@') {
            return Err("Enter a valid email address".into());
        }
    }
    Ok(())
}

fn optional_field(value: &Option<String>) -> Option<String> {
    value.as_ref().map(|v| v.trim()).filter(|v| !v.is_empty()).map(String::from)
}

#[tauri::command]
pub async fn list_suppliers<R: Runtime>(
    app: AppHandle<R>,
    search: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Page<SupplierRecord>, String> {
    let pool = db::pool(&app).await?;

    let pattern = search
        .as_deref()
        .map(|s| format!("%{}%", s.trim()))
        .filter(|p| p != "%%");
    let search_cond = if pattern.is_some() {
        " WHERE (s.name LIKE ? OR COALESCE(s.contact, '') LIKE ? OR COALESCE(s.phone, '') LIKE ?
               OR COALESCE(s.email, '') LIKE ? OR COALESCE(s.tax_id, '') LIKE ?)"
    } else {
        ""
    };
    let sql = format!(
        "SELECT s.id, s.name, s.contact, s.phone, s.email, s.address, s.tax_id, s.created_at,
                COUNT(si.id) AS invoice_count,
                COALESCE(SUM(si.total), 0.0) AS total_purchased,
                COALESCE(SUM(si.due_amount), 0.0) AS total_due
         FROM suppliers s
         LEFT JOIN supplier_invoices si ON si.supplier_id = s.id
         {search_cond}
         GROUP BY s.id
         ORDER BY s.name
         LIMIT ? OFFSET ?"
    );

    let mut query = sqlx::query(&sql);
    if let Some(p) = &pattern {
        for _ in 0..5 {
            query = query.bind(p);
        }
    }
    let rows = query
        .bind(limit.map(|l| l.max(1)).unwrap_or(-1))
        .bind(offset.unwrap_or(0).max(0))
        .fetch_all(&pool)
        .await
        .map_err(|e| e.to_string())?;

    let items: Vec<SupplierRecord> = rows
        .into_iter()
        .map(|row| -> Result<SupplierRecord, String> {
            Ok(SupplierRecord {
                id: row.try_get("id").map_err(|e| e.to_string())?,
                name: row.try_get("name").map_err(|e| e.to_string())?,
                contact: row.try_get("contact").map_err(|e| e.to_string())?,
                phone: row.try_get("phone").map_err(|e| e.to_string())?,
                email: row.try_get("email").map_err(|e| e.to_string())?,
                address: row.try_get("address").map_err(|e| e.to_string())?,
                tax_id: row.try_get("tax_id").map_err(|e| e.to_string())?,
                created_at: row.try_get("created_at").map_err(|e| e.to_string())?,
                invoice_count: row.try_get("invoice_count").map_err(|e| e.to_string())?,
                total_purchased: row.try_get("total_purchased").map_err(|e| e.to_string())?,
                total_due: row.try_get("total_due").map_err(|e| e.to_string())?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    let count_sql = format!("SELECT COUNT(*) FROM suppliers s{search_cond}");
    let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
    if let Some(p) = &pattern {
        for _ in 0..5 {
            count_query = count_query.bind(p);
        }
    }
    let total = count_query
        .fetch_one(&pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(Page { items, total })
}

#[derive(Debug, Serialize)]
pub struct SupplierInvoiceSummary {
    pub id: i64,
    pub invoice_no: String,
    pub date: String,
    pub total: f64,
    pub paid_amount: f64,
    pub due_amount: f64,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct SupplierDetail {
    pub supplier: SupplierRecord,
    pub invoices: Vec<SupplierInvoiceSummary>,
}

#[tauri::command]
pub async fn get_supplier<R: Runtime>(
    app: AppHandle<R>,
    supplier_id: i64,
) -> Result<SupplierDetail, String> {
    let pool = db::pool(&app).await?;
    let row = sqlx::query(
        "SELECT s.id, s.name, s.contact, s.phone, s.email, s.address, s.tax_id, s.created_at,
                (SELECT COUNT(*) FROM supplier_invoices WHERE supplier_id = s.id) AS invoice_count,
                COALESCE((SELECT SUM(total) FROM supplier_invoices WHERE supplier_id = s.id), 0.0) AS total_purchased,
                COALESCE((SELECT SUM(due_amount) FROM supplier_invoices WHERE supplier_id = s.id), 0.0) AS total_due
         FROM suppliers s
         WHERE s.id = ?",
    )
    .bind(supplier_id)
    .fetch_optional(&pool)
    .await
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "Supplier not found".to_string())?;

    let supplier = SupplierRecord {
        id: row.try_get("id").map_err(|e| e.to_string())?,
        name: row.try_get("name").map_err(|e| e.to_string())?,
        contact: row.try_get("contact").map_err(|e| e.to_string())?,
        phone: row.try_get("phone").map_err(|e| e.to_string())?,
        email: row.try_get("email").map_err(|e| e.to_string())?,
        address: row.try_get("address").map_err(|e| e.to_string())?,
        tax_id: row.try_get("tax_id").map_err(|e| e.to_string())?,
        created_at: row.try_get("created_at").map_err(|e| e.to_string())?,
        invoice_count: row.try_get("invoice_count").map_err(|e| e.to_string())?,
        total_purchased: row.try_get("total_purchased").map_err(|e| e.to_string())?,
        total_due: row.try_get("total_due").map_err(|e| e.to_string())?,
    };

    let invoices = sqlx::query(
        "SELECT id, invoice_no, date, total, paid_amount, due_amount, status
         FROM supplier_invoices
         WHERE supplier_id = ?
         ORDER BY date DESC, id DESC",
    )
    .bind(supplier_id)
    .fetch_all(&pool)
    .await
    .map_err(|e| e.to_string())?
    .into_iter()
    .map(|r| SupplierInvoiceSummary {
        id: r.get(0),
        invoice_no: r.get(1),
        date: r.get(2),
        total: r.get(3),
        paid_amount: r.get(4),
        due_amount: r.get(5),
        status: r.get(6),
    })
    .collect();

    Ok(SupplierDetail { supplier, invoices })
}

#[tauri::command]
pub async fn create_supplier<R: Runtime>(
    app: AppHandle<R>,
    input: SupplierInput,
) -> Result<i64, String> {
    validate_supplier(&input)?;
    let pool = db::pool(&app).await?;
    let result = sqlx::query(
        "INSERT INTO suppliers (name, contact, phone, email, address, tax_id)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(input.name.trim())
    .bind(optional_field(&input.contact))
    .bind(optional_field(&input.phone))
    .bind(optional_field(&input.email))
    .bind(optional_field(&input.address))
    .bind(optional_field(&input.tax_id))
    .execute(&pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(result.last_insert_rowid())
}

#[tauri::command]
pub async fn update_supplier<R: Runtime>(
    app: AppHandle<R>,
    supplier_id: i64,
    input: SupplierInput,
) -> Result<(), String> {
    validate_supplier(&input)?;
    let pool = db::pool(&app).await?;
    let result = sqlx::query(
        "UPDATE suppliers SET
            name = ?, contact = ?, phone = ?, email = ?, address = ?, tax_id = ?
         WHERE id = ?",
    )
    .bind(input.name.trim())
    .bind(optional_field(&input.contact))
    .bind(optional_field(&input.phone))
    .bind(optional_field(&input.email))
    .bind(optional_field(&input.address))
    .bind(optional_field(&input.tax_id))
    .bind(supplier_id)
    .execute(&pool)
    .await
    .map_err(|e| e.to_string())?;
    if result.rows_affected() == 0 {
        return Err("Supplier not found".into());
    }
    Ok(())
}

#[tauri::command]
pub async fn delete_supplier<R: Runtime>(
    app: AppHandle<R>,
    supplier_id: i64,
) -> Result<(), String> {
    let pool = db::pool(&app).await?;
    let invoices: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM supplier_invoices WHERE supplier_id = ?")
            .bind(supplier_id)
            .fetch_one(&pool)
            .await
            .map_err(|e| e.to_string())?;
    if invoices.0 > 0 {
        return Err(
            "This supplier has invoices on record and cannot be deleted. Edit their details instead."
                .into(),
        );
    }
    let result = sqlx::query("DELETE FROM suppliers WHERE id = ?")
        .bind(supplier_id)
        .execute(&pool)
        .await
        .map_err(|e| e.to_string())?;
    if result.rows_affected() == 0 {
        return Err("Supplier not found".into());
    }
    Ok(())
}
