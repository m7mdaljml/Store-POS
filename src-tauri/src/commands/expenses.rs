use serde::{Deserialize, Serialize};
use sqlx::Row;
use tauri::{AppHandle, Runtime};

use super::Page;
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
        "SELECT e.id, e.category_id, c.name AS category_name, e.amount, e.date, e.description, e.reference_no, u.full_name AS user_name
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

#[derive(Clone, Debug, Serialize)]
pub struct ExpenseRecord {
    pub kind: String, // 'in' (supplier invoice) or 'out' (outgoing expense)
    pub id: i64,
    pub ref_no: Option<String>,
    pub supplier_id: Option<i64>,
    pub supplier_name: Option<String>,
    pub date: String,
    pub amount: f64,
    pub paid_amount: f64,
    pub due_amount: f64,
    pub status: String,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ExpenseSummary {
    pub total_in: f64,
    pub total_out: f64,
    pub outstanding_due: f64,
    pub incoming_count: i64,
    pub outgoing_count: i64,
}

fn date_range_cond(from: &Option<String>, to: &Option<String>, column: &str) -> String {
    let mut conditions = String::new();
    if from.is_some() {
        conditions.push_str(&format!(" AND date({column}) >= date(?)"));
    }
    if to.is_some() {
        conditions.push_str(&format!(" AND date({column}) <= date(?)"));
    }
    conditions
}

async fn list_in_expenses(
    pool: &sqlx::SqlitePool,
    supplier_id: &Option<i64>,
    status: &Option<String>,
    from: &Option<String>,
    to: &Option<String>,
    pattern: &Option<String>,
) -> Result<Vec<ExpenseRecord>, String> {
    let range_cond = date_range_cond(from, to, "si.date");
    let search_cond = if pattern.is_some() {
        " AND (si.invoice_no LIKE ? OR s.name LIKE ? OR COALESCE(si.notes, '') LIKE ?)"
    } else {
        ""
    };
    let sql = format!(
        "SELECT si.id, si.invoice_no, si.supplier_id, s.name, si.date, si.total,
                si.paid_amount, si.due_amount, si.status, si.notes
         FROM supplier_invoices si
         JOIN suppliers s ON s.id = si.supplier_id
         WHERE (? IS NULL OR si.supplier_id = ?)
           AND (? IS NULL OR si.status = ?){search_cond}{range_cond}
         ORDER BY si.date DESC, si.id DESC"
    );

    let mut query = sqlx::query(&sql)
        .bind(supplier_id)
        .bind(supplier_id)
        .bind(status)
        .bind(status);
    if let Some(p) = pattern {
        query = query.bind(p).bind(p).bind(p);
    }
    if let Some(f) = from {
        query = query.bind(f);
    }
    if let Some(t) = to {
        query = query.bind(t);
    }
    let rows = query.fetch_all(pool).await.map_err(|e| e.to_string())?;

    rows.into_iter()
        .map(|row| {
            Ok(ExpenseRecord {
                kind: "in".into(),
                id: row.try_get("id").map_err(|e| e.to_string())?,
                ref_no: row.try_get("invoice_no").map_err(|e| e.to_string())?,
                supplier_id: row.try_get("supplier_id").map_err(|e| e.to_string())?,
                supplier_name: row.try_get("name").map_err(|e| e.to_string())?,
                date: row.try_get("date").map_err(|e| e.to_string())?,
                amount: row.try_get("total").map_err(|e| e.to_string())?,
                paid_amount: row.try_get("paid_amount").map_err(|e| e.to_string())?,
                due_amount: row.try_get("due_amount").map_err(|e| e.to_string())?,
                status: row.try_get("status").map_err(|e| e.to_string())?,
                notes: row.try_get("notes").map_err(|e| e.to_string())?,
            })
        })
        .collect()
}

async fn list_out_expenses(
    pool: &sqlx::SqlitePool,
    from: &Option<String>,
    to: &Option<String>,
    pattern: &Option<String>,
) -> Result<Vec<ExpenseRecord>, String> {
    let range_cond = date_range_cond(from, to, "e.date");
    let search_cond = if pattern.is_some() {
        " AND (COALESCE(e.description, '') LIKE ? OR COALESCE(e.reference_no, '') LIKE ?)"
    } else {
        ""
    };
    let sql = format!(
        "SELECT e.id, e.date, e.amount, e.description, e.reference_no
         FROM expense_out e
         WHERE 1=1{search_cond}{range_cond}
         ORDER BY e.date DESC, e.id DESC"
    );

    let mut query = sqlx::query(&sql);
    if let Some(p) = pattern {
        query = query.bind(p).bind(p);
    }
    if let Some(f) = from {
        query = query.bind(f);
    }
    if let Some(t) = to {
        query = query.bind(t);
    }
    let rows = query.fetch_all(pool).await.map_err(|e| e.to_string())?;

    rows.into_iter()
        .map(|row| {
            Ok(ExpenseRecord {
                kind: "out".into(),
                id: row.try_get("id").map_err(|e| e.to_string())?,
                ref_no: row.try_get("reference_no").map_err(|e| e.to_string())?,
                supplier_id: None,
                supplier_name: None,
                date: row.try_get("date").map_err(|e| e.to_string())?,
                amount: row.try_get("amount").map_err(|e| e.to_string())?,
                paid_amount: row.try_get("amount").map_err(|e| e.to_string())?,
                due_amount: 0.0,
                status: "recorded".into(),
                notes: row.try_get("description").map_err(|e| e.to_string())?,
            })
        })
        .collect()
}

pub async fn query_expenses(
    pool: &sqlx::SqlitePool,
    kind: &Option<String>,
    supplier_id: &Option<i64>,
    status: &Option<String>,
    from: &Option<String>,
    to: &Option<String>,
    search: &Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Page<ExpenseRecord>, String> {
    let show_in = kind.as_deref().map_or(true, |k| k == "in");
    let show_out = kind.as_deref().map_or(true, |k| k == "out");

    // The list unions incoming invoices and outgoing expenses, so the page
    // window is applied in Rust after merging and sorting.
    let pattern = search
        .as_deref()
        .map(|s| format!("%{}%", s.trim()))
        .filter(|p| p != "%%");

    let mut records = Vec::new();
    if show_in {
        records.extend(
            list_in_expenses(pool, supplier_id, status, from, to, &pattern).await?,
        );
    }
    if show_out {
        records.extend(list_out_expenses(pool, from, to, &pattern).await?);
    }
    records.sort_by(|a, b| b.date.cmp(&a.date).then(b.id.cmp(&a.id)));
    let total = records.len() as i64;

    if limit.is_none() && offset.unwrap_or(0) == 0 {
        return Ok(Page {
            items: records,
            total,
        });
    }
    let start = offset.unwrap_or(0).max(0) as usize;
    if start >= records.len() {
        return Ok(Page {
            items: Vec::new(),
            total,
        });
    }
    match limit {
        Some(l) => {
            let end = (start + l.max(1) as usize).min(records.len());
            Ok(Page {
                items: records[start..end].to_vec(),
                total,
            })
        }
        None => Ok(Page {
            items: records[start..].to_vec(),
            total,
        }),
    }
}

#[tauri::command]
pub async fn list_expenses<R: Runtime>(
    app: AppHandle<R>,
    kind: Option<String>,
    supplier_id: Option<i64>,
    status: Option<String>,
    from: Option<String>,
    to: Option<String>,
    search: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Page<ExpenseRecord>, String> {
    let kind = kind.map(|k| k.to_lowercase());
    if let Some(k) = &kind {
        if k != "in" && k != "out" {
            return Err("Kind must be 'in' or 'out'".into());
        }
    }
    let pool = db::pool(&app).await?;
    query_expenses(
        &pool, &kind, &supplier_id, &status, &from, &to, &search, limit, offset,
    )
    .await
}

#[tauri::command]
pub async fn expense_summary<R: Runtime>(
    app: AppHandle<R>,
    from: Option<String>,
    to: Option<String>,
) -> Result<ExpenseSummary, String> {
    let pool = db::pool(&app).await?;
    compute_expense_summary(&pool, &from, &to).await
}

pub async fn compute_expense_summary(
    pool: &sqlx::SqlitePool,
    from: &Option<String>,
    to: &Option<String>,
) -> Result<ExpenseSummary, String> {
    let in_cond = date_range_cond(from, to, "date");
    let in_sql = format!(
        "SELECT COUNT(*), COALESCE(SUM(total), 0.0) FROM supplier_invoices WHERE 1=1{in_cond}"
    );
    let mut in_query = sqlx::query_as(&in_sql);
    if let Some(f) = from {
        in_query = in_query.bind(f);
    }
    if let Some(t) = to {
        in_query = in_query.bind(t);
    }
    let (incoming_count, total_in): (i64, f64) =
        in_query.fetch_one(pool).await.map_err(|e| e.to_string())?;

    let out_cond = date_range_cond(from, to, "date");
    let out_sql = format!(
        "SELECT COUNT(*), COALESCE(SUM(amount), 0.0) FROM expense_out WHERE 1=1{out_cond}"
    );
    let mut out_query = sqlx::query_as(&out_sql);
    if let Some(f) = from {
        out_query = out_query.bind(f);
    }
    if let Some(t) = to {
        out_query = out_query.bind(t);
    }
    let (outgoing_count, total_out): (i64, f64) =
        out_query.fetch_one(pool).await.map_err(|e| e.to_string())?;

    let outstanding_due: f64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(due_amount), 0.0) FROM supplier_invoices WHERE status != 'paid'",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(ExpenseSummary {
        total_in,
        total_out,
        outstanding_due,
        incoming_count,
        outgoing_count,
    })
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
    .bind(format!(
        "Recorded outgoing expense of {}",
        crate::format::money(input.amount)
    ))
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

const EXPORT_HEADERS: [&str; 9] = [
    "Type", "Reference", "Supplier", "Date", "Notes", "Amount", "Paid", "Due", "Status",
];

fn expense_records_to_rows(records: &[ExpenseRecord]) -> Vec<Vec<String>> {
    records
        .iter()
        .map(|r| {
            vec![
                if r.kind == "in" {
                    "Incoming".into()
                } else {
                    "Outgoing".into()
                },
                r.ref_no.clone().unwrap_or_default(),
                r.supplier_name.clone().unwrap_or_default(),
                r.date.clone(),
                r.notes.clone().unwrap_or_default(),
                format!("{:.2}", r.amount),
                if r.kind == "in" {
                    format!("{:.2}", r.paid_amount)
                } else {
                    String::new()
                },
                if r.kind == "in" {
                    format!("{:.2}", r.due_amount)
                } else {
                    String::new()
                },
                r.status.clone(),
            ]
        })
        .collect()
}

/// Exports the filtered expense list to an `.xlsx` file at the given path.
#[tauri::command]
pub async fn export_expenses<R: Runtime>(
    app: AppHandle<R>,
    path: String,
    kind: Option<String>,
    supplier_id: Option<i64>,
    status: Option<String>,
    from: Option<String>,
    to: Option<String>,
) -> Result<(), String> {
    let kind = kind.map(|k| k.to_lowercase());
    if let Some(k) = &kind {
        if k != "in" && k != "out" {
            return Err("Kind must be 'in' or 'out'".into());
        }
    }
    let pool = db::pool(&app).await?;
    let records =
        query_expenses(&pool, &kind, &supplier_id, &status, &from, &to, &None, None, None)
            .await?
            .items;
    let rows = expense_records_to_rows(&records);
    crate::export::write_xlsx(
        std::path::Path::new(&path),
        "Expenses",
        &EXPORT_HEADERS,
        &rows,
    )
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
            "CREATE TABLE suppliers (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, contact TEXT, phone TEXT, email TEXT, address TEXT, tax_id TEXT, created_at TEXT NOT NULL DEFAULT (datetime('now')))",
            "CREATE TABLE supplier_invoices (id INTEGER PRIMARY KEY AUTOINCREMENT, invoice_no TEXT NOT NULL, supplier_id INTEGER NOT NULL, date TEXT NOT NULL, total REAL NOT NULL DEFAULT 0, paid_amount REAL NOT NULL DEFAULT 0, due_amount REAL NOT NULL DEFAULT 0, status TEXT NOT NULL DEFAULT 'unpaid', notes TEXT, user_id INTEGER, created_at TEXT NOT NULL DEFAULT (datetime('now')))",
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

    #[tokio::test]
    async fn list_expenses_filters_and_summary() {
        let path = temp_db("expense-filter");
        create_schema(&path).await;
        let pool = db::connect(&path).await.unwrap();

        let empty_summary = compute_expense_summary(&pool, &None, &None).await.unwrap();
        assert_eq!(empty_summary.total_in, 0.0);
        assert_eq!(empty_summary.total_out, 0.0);
        assert_eq!(empty_summary.outstanding_due, 0.0);

        let supplier_a = sqlx::query("INSERT INTO suppliers (name) VALUES ('Supplier A')")
            .execute(&pool)
            .await
            .unwrap()
            .last_insert_rowid();
        let supplier_b = sqlx::query("INSERT INTO suppliers (name) VALUES ('Supplier B')")
            .execute(&pool)
            .await
            .unwrap()
            .last_insert_rowid();

        sqlx::query(
            "INSERT INTO supplier_invoices (invoice_no, supplier_id, date, total, paid_amount, due_amount, status)
             VALUES ('PI-000001', ?, '2026-08-01', 100, 100, 0, 'paid')",
        )
        .bind(supplier_a)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO supplier_invoices (invoice_no, supplier_id, date, total, paid_amount, due_amount, status)
             VALUES ('PI-000002', ?, '2026-08-10', 200, 50, 150, 'partial')",
        )
        .bind(supplier_a)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO supplier_invoices (invoice_no, supplier_id, date, total, paid_amount, due_amount, status)
             VALUES ('PI-000003', ?, '2026-09-01', 300, 0, 300, 'unpaid')",
        )
        .bind(supplier_b)
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO expense_out (amount, date, description) VALUES (50, '2026-08-05', 'Rent')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO expense_out (amount, date, description) VALUES (25, '2026-09-02', 'Utilities')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let all_in = list_in_expenses(&pool, &None, &None, &None, &None, &None)
            .await
            .unwrap();
        assert_eq!(all_in.len(), 3);

        let by_supplier = list_in_expenses(
            &pool,
            &Some(supplier_a),
            &None,
            &None,
            &None,
            &None,
        )
        .await
        .unwrap();
        assert_eq!(by_supplier.len(), 2);

        let paid_only = list_in_expenses(
            &pool,
            &None,
            &Some("paid".into()),
            &None,
            &None,
            &None,
        )
        .await
        .unwrap();
        assert_eq!(paid_only.len(), 1);
        assert_eq!(paid_only[0].ref_no.as_deref(), Some("PI-000001"));

        let august = list_in_expenses(
            &pool,
            &None,
            &None,
            &Some("2026-08-01".into()),
            &Some("2026-08-31".into()),
            &None,
        )
        .await
        .unwrap();
        assert_eq!(august.len(), 2, "both August invoices");

        let out = list_out_expenses(
            &pool,
            &Some("2026-08-01".into()),
            &Some("2026-08-31".into()),
            &None,
        )
        .await
        .unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].kind, "out");
        assert_eq!(out[0].amount, 50.0);
        assert_eq!(out[0].status, "recorded");

        let summary = compute_expense_summary(
            &pool,
            &Some("2026-08-01".into()),
            &Some("2026-08-31".into()),
        )
        .await
        .unwrap();
        assert_eq!(summary.total_in, 300.0, "100 + 200 in August");
        assert_eq!(summary.incoming_count, 2);
        assert_eq!(summary.total_out, 50.0, "only August outgoing");
        assert_eq!(summary.outgoing_count, 1);
        assert_eq!(summary.outstanding_due, 450.0, "150 + 300 global dues");

        pool.close().await;
        std::fs::remove_file(&path).ok();
    }

    #[tokio::test]
    async fn export_expenses_writes_xlsx() {
        let path = temp_db("expense-export");
        create_schema(&path).await;
        let pool = db::connect(&path).await.unwrap();

        let supplier_a = sqlx::query("INSERT INTO suppliers (name) VALUES ('Supplier A')")
            .execute(&pool)
            .await
            .unwrap()
            .last_insert_rowid();
        sqlx::query(
            "INSERT INTO supplier_invoices (invoice_no, supplier_id, date, total, paid_amount, due_amount, status)
             VALUES ('PI-000001', ?, '2026-08-01', 100, 100, 0, 'paid')",
        )
        .bind(supplier_a)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO expense_out (amount, date, description) VALUES (50, '2026-08-05', 'Rent')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let records =
            query_expenses(&pool, &None, &None, &None, &None, &None, &None, None, None)
                .await
                .unwrap()
                .items;
        assert_eq!(records.len(), 2);

        let xlsx_path = temp_db("export-file").with_extension("xlsx");
        let rows = expense_records_to_rows(&records);
        crate::export::write_xlsx(
            &xlsx_path,
            "Expenses",
            &EXPORT_HEADERS,
            &rows,
        )
        .unwrap();

        let meta = std::fs::metadata(&xlsx_path).unwrap();
        assert!(meta.len() > 0, "xlsx file should not be empty");
        assert_eq!(rows[0].len(), EXPORT_HEADERS.len());
        assert_eq!(rows[0][0], "Outgoing", "newer record sorts first");
        assert_eq!(rows[1][0], "Incoming");
        assert_eq!(rows[1][4], "", "incoming notes come from invoice notes");

        std::fs::remove_file(xlsx_path).ok();
        pool.close().await;
        std::fs::remove_file(&path).ok();
    }
}
