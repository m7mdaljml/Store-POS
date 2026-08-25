use serde::{Deserialize, Serialize};
use sqlx::Row;
use tauri::{AppHandle, Runtime};

use super::Page;
use crate::db;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenSessionInput {
    pub opening_cash: f64,
    pub user_id: i64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloseSessionInput {
    pub session_id: i64,
    pub closing_cash: f64,
    pub user_id: i64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListSessionsInput {
    pub status: Option<String>,
    pub search: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaleSession {
    pub id: i64,
    pub user_name: Option<String>,
    pub opened_at: String,
    pub closed_at: Option<String>,
    pub opening_cash: f64,
    pub closing_cash: Option<f64>,
    /// Expected drawer cash = opening + cash taken in - change given back.
    pub expected_cash: Option<f64>,
    /// Actual - expected (positive = over, negative = short).
    pub variance: Option<f64>,
    pub status: String,
    /// Completed sales recorded against the session.
    pub sales_count: i64,
    pub sales_total: f64,
    /// Cash payments received on completed sales in the session.
    pub cash_paid: f64,
    /// Change given back on completed sales in the session.
    pub change_given: f64,
}

const SESSION_SELECT: &str = "
    SELECT ss.id, u.full_name, ss.opened_at, ss.closed_at,
           ss.opening_cash, ss.closing_cash, ss.expected_cash, ss.variance, ss.status,
           (SELECT COUNT(*) FROM sales s WHERE s.session_id = ss.id AND s.status = 'completed') AS sales_count,
           (SELECT COALESCE(SUM(s.total - COALESCE((SELECT SUM(r.amount) FROM refunds r WHERE r.sale_id = s.id), 0.0)), 0.0)
              FROM sales s WHERE s.session_id = ss.id AND s.status = 'completed') AS sales_total,
           (SELECT COALESCE(SUM(p.amount), 0.0) FROM payments p JOIN sales s ON s.id = p.sale_id
             WHERE s.session_id = ss.id AND s.status = 'completed' AND p.method = 'cash') AS cash_paid,
           (SELECT COALESCE(SUM(s.change_given), 0.0) FROM sales s WHERE s.session_id = ss.id AND s.status = 'completed') AS change_given
    FROM sale_sessions ss
    LEFT JOIN users u ON u.id = ss.user_id
";

fn map_session(row: &sqlx::sqlite::SqliteRow) -> Result<SaleSession, String> {
    Ok(SaleSession {
        id: row.try_get("id").map_err(|e| e.to_string())?,
        user_name: row.try_get("full_name").map_err(|e| e.to_string())?,
        opened_at: row.try_get("opened_at").map_err(|e| e.to_string())?,
        closed_at: row.try_get("closed_at").map_err(|e| e.to_string())?,
        opening_cash: row.try_get("opening_cash").map_err(|e| e.to_string())?,
        closing_cash: row.try_get("closing_cash").map_err(|e| e.to_string())?,
        expected_cash: row.try_get("expected_cash").map_err(|e| e.to_string())?,
        variance: row.try_get("variance").map_err(|e| e.to_string())?,
        status: row.try_get("status").map_err(|e| e.to_string())?,
        sales_count: row.try_get("sales_count").map_err(|e| e.to_string())?,
        sales_total: row.try_get("sales_total").map_err(|e| e.to_string())?,
        cash_paid: row.try_get("cash_paid").map_err(|e| e.to_string())?,
        change_given: row.try_get("change_given").map_err(|e| e.to_string())?,
    })
}

/// Opens the cash register for the shift. Only one session may be open at a time.
pub async fn insert_session(
    pool: &sqlx::SqlitePool,
    input: OpenSessionInput,
) -> Result<SaleSession, String> {
    if input.opening_cash < 0.0 {
        return Err("Opening cash cannot be negative".into());
    }

    let open: Option<(i64,)> =
        sqlx::query_as("SELECT id FROM sale_sessions WHERE status = 'open' LIMIT 1")
            .fetch_optional(pool)
            .await
            .map_err(|e| e.to_string())?;
    if let Some((id,)) = open {
        return Err(format!("A register session is already open (session {id})"));
    }

    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    let result = sqlx::query(
        "INSERT INTO sale_sessions (user_id, opening_cash, status) VALUES (?, ?, 'open')",
    )
    .bind(input.user_id)
    .bind(input.opening_cash)
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    let id = result.last_insert_rowid();

    sqlx::query(
        "INSERT INTO audit_log (user_id, action, entity_type, entity_id, details)
         VALUES (?, 'session.open', 'sale_session', ?, ?)",
    )
    .bind(input.user_id)
    .bind(id)
    .bind(format!("Opened register with opening cash {}", input.opening_cash))
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    tx.commit().await.map_err(|e| e.to_string())?;

    let rows = sqlx::query(&format!("{SESSION_SELECT} WHERE ss.id = ?"))
        .bind(id)
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;
    let row = rows.into_iter().next().ok_or("Session not found")?;
    map_session(&row)
}

/// Closes the register: counts the drawer, computes the expected cash and the
/// variance (actual - expected), then records it on the session.
pub async fn finalize_session(
    pool: &sqlx::SqlitePool,
    input: CloseSessionInput,
) -> Result<SaleSession, String> {
    if input.closing_cash < 0.0 {
        return Err("Closing cash cannot be negative".into());
    }

    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    let current: Option<(String, f64)> =
        sqlx::query_as("SELECT status, opening_cash FROM sale_sessions WHERE id = ?")
            .bind(input.session_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
    let (status, opening_cash) =
        current.ok_or_else(|| format!("Session {} not found", input.session_id))?;
    if status != "open" {
        return Err("Session is not open".into());
    }

    let cash_paid: (f64,) = sqlx::query_as(
        "SELECT COALESCE(SUM(p.amount), 0.0)
         FROM payments p
         JOIN sales s ON s.id = p.sale_id
         WHERE s.session_id = ? AND s.status = 'completed' AND p.method = 'cash'",
    )
    .bind(input.session_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    let change_given: (f64,) = sqlx::query_as(
        "SELECT COALESCE(SUM(change_given), 0.0)
         FROM sales WHERE session_id = ? AND status = 'completed'",
    )
    .bind(input.session_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    let expected = opening_cash + cash_paid.0 - change_given.0;
    let variance = input.closing_cash - expected;

    sqlx::query(
        "UPDATE sale_sessions
         SET status = 'closed', closed_at = datetime('now'), closed_by = ?,
             closing_cash = ?, expected_cash = ?, variance = ?, notes = ?
         WHERE id = ?",
    )
    .bind(input.user_id)
    .bind(input.closing_cash)
    .bind(expected)
    .bind(variance)
    .bind(format!("Expected {}, counted {}", expected, input.closing_cash))
    .bind(input.session_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query(
        "INSERT INTO audit_log (user_id, action, entity_type, entity_id, details)
         VALUES (?, 'session.close', 'sale_session', ?, ?)",
    )
    .bind(input.user_id)
    .bind(input.session_id)
    .bind(format!(
        "Closed register: expected {}, actual {}, variance {}",
        expected, input.closing_cash, variance
    ))
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    tx.commit().await.map_err(|e| e.to_string())?;

    let rows = sqlx::query(&format!("{SESSION_SELECT} WHERE ss.id = ?"))
        .bind(input.session_id)
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;
    let row = rows.into_iter().next().ok_or("Session not found")?;
    map_session(&row)
}

/// Returns the currently open session, if any.
pub async fn query_open_session(
    pool: &sqlx::SqlitePool,
) -> Result<Option<SaleSession>, String> {
    let rows = sqlx::query(&format!("{SESSION_SELECT} WHERE ss.status = 'open' LIMIT 1"))
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;
    match rows.into_iter().next() {
        Some(row) => Ok(Some(map_session(&row)?)),
        None => Ok(None),
    }
}

/// Lists session history, optionally filtered by status.
pub async fn query_sessions(
    pool: &sqlx::SqlitePool,
    input: ListSessionsInput,
) -> Result<Page<SaleSession>, String> {
    let limit = input.limit.unwrap_or(50).max(1).min(500);
    let offset = input.offset.unwrap_or(0).max(0);

    let pattern = input
        .search
        .as_deref()
        .map(|s| format!("%{}%", s.trim()))
        .filter(|p| p != "%%");
    let search_cond = if pattern.is_some() {
        " AND (COALESCE(u.full_name, '') LIKE ? OR COALESCE(u.username, '') LIKE ?
              OR CAST(ss.id AS TEXT) LIKE ?)"
    } else {
        ""
    };
    let sql = format!(
        "{SESSION_SELECT} WHERE (? IS NULL OR ss.status = ?){search_cond}
         ORDER BY ss.id DESC LIMIT ? OFFSET ?"
    );
    let mut query = sqlx::query(&sql)
        .bind(&input.status)
        .bind(&input.status);
    if let Some(p) = &pattern {
        query = query.bind(p).bind(p).bind(p);
    }
    let rows = query
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;
    let total: i64 = {
        let count_sql = format!(
            "SELECT COUNT(*) FROM sale_sessions ss
             LEFT JOIN users u ON u.id = ss.user_id
             WHERE (? IS NULL OR ss.status = ?){search_cond}"
        );
        let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql)
            .bind(&input.status)
            .bind(&input.status);
        if let Some(p) = &pattern {
            count_query = count_query.bind(p).bind(p).bind(p);
        }
        count_query
            .fetch_one(pool)
            .await
            .map_err(|e| e.to_string())?
    };

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        out.push(map_session(&row)?);
    }
    Ok(Page { items: out, total })
}

#[tauri::command]
pub async fn open_session<R: Runtime>(
    app: AppHandle<R>,
    input: OpenSessionInput,
) -> Result<SaleSession, String> {
    let pool = db::pool(&app).await?;
    insert_session(&pool, input).await
}

#[tauri::command]
pub async fn close_session<R: Runtime>(
    app: AppHandle<R>,
    input: CloseSessionInput,
) -> Result<SaleSession, String> {
    let pool = db::pool(&app).await?;
    finalize_session(&pool, input).await
}

#[tauri::command]
pub async fn get_open_session<R: Runtime>(
    app: AppHandle<R>,
) -> Result<Option<SaleSession>, String> {
    let pool = db::pool(&app).await?;
    query_open_session(&pool).await
}

#[tauri::command]
pub async fn list_sessions<R: Runtime>(
    app: AppHandle<R>,
    input: Option<ListSessionsInput>,
) -> Result<Page<SaleSession>, String> {
    let pool = db::pool(&app).await?;
    query_sessions(
        &pool,
        input.unwrap_or(ListSessionsInput {
            status: None,
            search: None,
            limit: None,
            offset: None,
        }),
    )
    .await
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
        CREATE TABLE sales (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          sale_no TEXT UNIQUE NOT NULL,
          session_id INTEGER,
          user_id INTEGER NOT NULL,
          subtotal REAL NOT NULL DEFAULT 0,
          discount REAL NOT NULL DEFAULT 0,
          tax REAL NOT NULL DEFAULT 0,
          total REAL NOT NULL DEFAULT 0,
          paid_amount REAL NOT NULL DEFAULT 0,
          change_given REAL NOT NULL DEFAULT 0,
          status TEXT NOT NULL DEFAULT 'completed',
          created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE TABLE payments (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          sale_id INTEGER NOT NULL,
          method TEXT NOT NULL,
          amount REAL NOT NULL,
          reference TEXT,
          created_at TEXT NOT NULL DEFAULT (datetime('now'))
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
        pool
    }

    async fn open(pool: &sqlx::SqlitePool) -> SaleSession {
        insert_session(
            pool,
            OpenSessionInput {
                opening_cash: 100.0,
                user_id: 1,
            },
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn open_session_records_and_rejects_second_open() {
        let pool = mem_pool().await;
        sqlx::query("INSERT INTO users (username, full_name) VALUES ('cashier', 'Sam')")
            .execute(&pool)
            .await
            .unwrap();

        let s = open(&pool).await;
        assert_eq!(s.status, "open");
        assert_eq!(s.user_name.as_deref(), Some("Sam"));
        assert!((s.opening_cash - 100.0).abs() < 0.001);
        assert!(s.closed_at.is_none());

        let err = insert_session(
            &pool,
            OpenSessionInput {
                opening_cash: 50.0,
                user_id: 1,
            },
        )
        .await
        .unwrap_err();
        assert!(err.contains("already open"));
    }

    #[tokio::test]
    async fn close_session_computes_expected_and_variance() {
        let pool = mem_pool().await;
        let s = open(&pool).await;

        // Cash sale: 60 paid, 10 change back -> drawer +50.
        sqlx::query(
            "INSERT INTO sales (sale_no, session_id, user_id, total, paid_amount, change_given, status)
             VALUES ('S-000001', ?, 1, 50, 60, 10, 'completed')",
        )
        .bind(s.id)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO payments (sale_id, method, amount) VALUES (1, 'cash', 60)")
            .execute(&pool)
            .await
            .unwrap();

        // Card sale: 30 paid, no cash touches the drawer.
        sqlx::query(
            "INSERT INTO sales (sale_no, session_id, user_id, total, paid_amount, change_given, status)
             VALUES ('S-000002', ?, 1, 30, 30, 0, 'completed')",
        )
        .bind(s.id)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO payments (sale_id, method, amount) VALUES (2, 'card', 30)")
            .execute(&pool)
            .await
            .unwrap();

        // Voided sale must NOT count: cash was already in the drawer though.
        sqlx::query(
            "INSERT INTO sales (sale_no, session_id, user_id, total, paid_amount, change_given, status)
             VALUES ('S-000003', ?, 1, 10, 10, 0, 'voided')",
        )
        .bind(s.id)
        .execute(&pool)
        .await
        .unwrap();

        // expected = 100 + 60 - 10 = 150
        let closed = finalize_session(
            &pool,
            CloseSessionInput {
                session_id: s.id,
                closing_cash: 150.0,
                user_id: 1,
            },
        )
        .await
        .unwrap();
        assert_eq!(closed.status, "closed");
        assert!((closed.expected_cash.unwrap() - 150.0).abs() < 0.001);
        assert!((closed.variance.unwrap()).abs() < 0.001);
        assert_eq!(closed.sales_count, 2);
        assert!((closed.sales_total - 80.0).abs() < 0.001);
        assert!((closed.cash_paid - 60.0).abs() < 0.001);
        assert!((closed.change_given - 10.0).abs() < 0.001);
        assert!(closed.closed_at.is_some());

        // A short drawer records a negative variance.
        let s2 = open(&pool).await;
        let closed2 = finalize_session(
            &pool,
            CloseSessionInput {
                session_id: s2.id,
                closing_cash: 95.0,
                user_id: 1,
            },
        )
        .await
        .unwrap();
        assert!((closed2.variance.unwrap() + 5.0).abs() < 0.001);

        // Closing again fails.
        let err = finalize_session(
            &pool,
            CloseSessionInput {
                session_id: s.id,
                closing_cash: 150.0,
                user_id: 1,
            },
        )
        .await
        .unwrap_err();
        assert!(err.contains("not open"));
    }

    #[tokio::test]
    async fn open_session_validation_and_queries() {
        let pool = mem_pool().await;
        assert!(query_open_session(&pool).await.unwrap().is_none());

        let err = insert_session(
            &pool,
            OpenSessionInput {
                opening_cash: -1.0,
                user_id: 1,
            },
        )
        .await
        .unwrap_err();
        assert!(err.contains("Opening cash"));

        let err = finalize_session(
            &pool,
            CloseSessionInput {
                session_id: 999,
                closing_cash: 0.0,
                user_id: 1,
            },
        )
        .await
        .unwrap_err();
        assert!(err.contains("not found"));

        let s = open(&pool).await;
        let current = query_open_session(&pool).await.unwrap().unwrap();
        assert_eq!(current.id, s.id);

        let all = query_sessions(
            &pool,
            ListSessionsInput {
                status: None,
                search: None,
                limit: None,
                offset: None,
            },
        )
        .await
        .unwrap()
        .items;
        assert_eq!(all.len(), 1);

        let closed_only = query_sessions(
            &pool,
            ListSessionsInput {
                status: Some("closed".into()),
                search: None,
                limit: None,
                offset: None,
            },
        )
        .await
        .unwrap()
        .items;
        assert!(closed_only.is_empty());
    }

    /// A refund issued during the session reduces its net sales total; a cash
    /// refund also lowers the expected drawer through the negative payment row.
    #[tokio::test]
    async fn refund_reduces_session_sales_total_and_cash() {
        let pool = mem_pool().await;
        let s = open(&pool).await;

        // Cash sale of 40.
        sqlx::query(
            "INSERT INTO sales (sale_no, session_id, user_id, total, paid_amount, change_given, status)
             VALUES ('S-000001', ?, 1, 40, 40, 0, 'completed')",
        )
        .bind(s.id)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO payments (sale_id, method, amount) VALUES (1, 'cash', 40)")
            .execute(&pool)
            .await
            .unwrap();

        // Cash refund of 15 against it: negative payment row + refunds row.
        sqlx::query(
            "INSERT INTO refunds (sale_no, sale_id, session_id, method, amount) VALUES ('R-000001', 1, ?, 'cash', 15)",
        )
        .bind(s.id)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO payments (sale_id, method, amount) VALUES (1, 'cash', -15)")
            .execute(&pool)
            .await
            .unwrap();

        let current = query_open_session(&pool).await.unwrap().unwrap();
        assert!((current.sales_total - 25.0).abs() < 0.001);
        assert!((current.cash_paid - 25.0).abs() < 0.001);

        // expected = 100 + 40 - 15 = 125
        let closed = finalize_session(
            &pool,
            CloseSessionInput {
                session_id: s.id,
                closing_cash: 125.0,
                user_id: 1,
            },
        )
        .await
        .unwrap();
        assert!((closed.expected_cash.unwrap() - 125.0).abs() < 0.001);
        assert!((closed.variance.unwrap()).abs() < 0.001);
        assert!((closed.sales_total - 25.0).abs() < 0.001);
    }
}
