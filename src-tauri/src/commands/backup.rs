use crate::db;
use crate::export::{self, SheetData};
use serde::Serialize;
use sqlx::sqlite::SqlitePool;
use sqlx::{Column, Row};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager, Runtime};

const SQLITE_HEADER: &[u8; 16] = b"SQLite format 3\x00";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupRow {
    pub id: i64,
    pub path: String,
    pub size_bytes: i64,
    pub kind: String,
    pub created_at: String,
}

/// UTC timestamp used in backup filenames: YYYYMMDD-HHMMSS (no chrono dep).
fn timestamp_label() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (h, m, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    // Howard Hinnant's civil_from_days algorithm.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mth = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mth <= 2 { y + 1 } else { y };
    format!("{y:04}{mth:02}{d:02}-{h:02}{m:02}{s:02}")
}

// NOTE: the `backups` table ships with the initial schema (001_initial.sql):
//   id INTEGER PK, file_path TEXT NOT NULL, size_bytes INTEGER,
//   status TEXT NOT NULL DEFAULT 'completed', created_at TEXT
// We reuse it as-is: `status` carries the entry kind
// ('manual' | 'auto' | 'pre_restore').
fn ensure_backups_table_sql() -> &'static str {
    "CREATE TABLE IF NOT EXISTS backups (
       id          INTEGER PRIMARY KEY AUTOINCREMENT,
       file_path   TEXT UNIQUE NOT NULL,
       size_bytes  INTEGER NOT NULL DEFAULT 0,
       status      TEXT NOT NULL DEFAULT 'completed',
       created_at  TEXT NOT NULL DEFAULT (datetime('now'))
     )"
}

async fn ensure_backups_table(pool: &SqlitePool) -> Result<(), String> {
    sqlx::query(ensure_backups_table_sql())
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Reads the retention policy setting (F8.3). Defaults to keeping 5 backups.
async fn retention_setting(pool: &SqlitePool) -> Result<i64, String> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT value FROM settings WHERE key = 'backup_retention'")
            .fetch_optional(pool)
            .await
            .map_err(|e| e.to_string())?;
    let n = row
        .and_then(|(v,)| v.parse::<i64>().ok())
        .unwrap_or(5)
        .max(1);
    Ok(n)
}

fn managed_backup_file(p: &Path) -> bool {
    p.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.starts_with("store-backup-") || n.starts_with("pre-restore-"))
}

/// Deletes backup records beyond the newest `keep`, removing their files when
/// they are ours (managed naming). Returns the number of pruned entries.
pub async fn prune_backups(pool: &SqlitePool, keep: i64) -> Result<usize, String> {
    ensure_backups_table(pool).await?;
    let stale: Vec<(i64, String)> = sqlx::query_as(
        "SELECT id, file_path FROM backups ORDER BY datetime(created_at) DESC, id DESC LIMIT -1 OFFSET ?",
    )
    .bind(keep)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let mut pruned = 0usize;
    for (id, path) in stale {
        sqlx::query("DELETE FROM backups WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
        let p = PathBuf::from(&path);
        if managed_backup_file(&p) && fs::remove_file(&p).is_ok() {
            pruned += 1;
        } else if !managed_backup_file(&p) {
            pruned += 1; // foreign/missing file still counts as removed entry
        } else if !p.exists() {
            pruned += 1;
        }
    }
    Ok(pruned)
}
/// Creates a consistent snapshot of the database `pool` is connected to,
/// writing it inside `dir` via VACUUM INTO and logging it in the backups
/// table. Safe to run while other connections are active.
pub async fn backup_to_file(pool: &SqlitePool, dir: &Path, kind: &str) -> Result<BackupRow, String> {
    ensure_backups_table(pool).await?;
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;

    let target = dir.join(format!("store-backup-{}.db", timestamp_label()));
    let target_str = target.to_string_lossy().to_string();

    // VACUUM INTO writes a fully consistent copy even while writers are live.
    sqlx::query("VACUUM INTO ?1")
        .bind(&target_str)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    let size = fs::metadata(&target).map(|m| m.len() as i64).unwrap_or(0);

    let row: (i64, String, i64, String, String) = sqlx::query_as(
        "INSERT INTO backups (file_path, size_bytes, status) VALUES (?, ?, ?)
         RETURNING id, file_path, size_bytes, status, created_at",
    )
    .bind(&target_str)
    .bind(size)
    .bind(kind)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(BackupRow {
        id: row.0,
        path: row.1,
        size_bytes: row.2,
        kind: row.3,
        created_at: row.4,
    })
}

/// Validates that `source` looks like a SQLite database file.
fn is_sqlite_file(path: &Path) -> Result<(), String> {
    let mut header = [0u8; 16];
    use std::io::Read;
    let mut f = fs::File::open(path).map_err(|e| e.to_string())?;
    f.read_exact(&mut header)
        .map_err(|_| "Selected file is too small to be a database".to_string())?;
    if &header != SQLITE_HEADER {
        return Err("Selected file is not a SQLite database".to_string());
    }
    Ok(())
}

/// Removes WAL/SHM sidecars of a database file. Must run whenever the main
/// file is replaced externally — keeping stale sidecars from the previous
/// generation next to a new file corrupts it on open.
fn remove_sidecars(db_file: &Path) {
    for suffix in ["-wal", "-shm"] {
        let mut name = db_file.as_os_str().to_os_string();
        name.push(suffix);
        let _ = fs::remove_file(PathBuf::from(name));
    }
}

/// Restores `source` over `live` (F8.5). Takes a logged safety copy of the
/// current database first, closes the pool, then replaces the file.
/// The caller must have closed every other connection beforehand.
pub async fn restore_from_file(
    live: &Path,
    source: &Path,
    safety_dir: &Path,
) -> Result<BackupRow, String> {
    is_sqlite_file(source)?;
    fs::create_dir_all(safety_dir).map_err(|e| e.to_string())?;

    let pool = db::connect(live).await?;
    ensure_backups_table(&pool).await?;

    // Safety copy of the current state before it gets overwritten.
    let safety_path = safety_dir.join(format!("pre-restore-{}.db", timestamp_label()));
    let safety_str = safety_path.to_string_lossy().to_string();
    sqlx::query("VACUUM INTO ?1")
        .bind(&safety_str)
        .execute(&pool)
        .await
        .map_err(|e| e.to_string())?;
    let size = fs::metadata(&safety_path).map(|m| m.len() as i64).unwrap_or(0);
    let row: (i64, String, i64, String, String) = sqlx::query_as(
        "INSERT INTO backups (file_path, size_bytes, status) VALUES (?, ?, 'pre_restore')
         RETURNING id, file_path, size_bytes, status, created_at",
    )
    .bind(&safety_str)
    .bind(size)
    .fetch_one(&pool)
    .await
    .map_err(|e| e.to_string())?;
    pool.close().await;

    // Drop sidecars from the OLD generation, then replace the main file.
    remove_sidecars(live);
    fs::copy(source, live).map_err(|e| e.to_string())?;

    Ok(BackupRow {
        id: row.0,
        path: row.1,
        size_bytes: row.2,
        kind: row.3,
        created_at: row.4,
    })
}

/* ------------------------------------------------------------------ */
/* Tauri commands                                                      */
/* ------------------------------------------------------------------ */

/// F8.2/F8.3 — snapshot the database into `dir` (or `<config>/backups`) and
/// apply the retention policy afterwards.
#[tauri::command]
pub async fn create_backup<R: Runtime>(
    app: AppHandle<R>,
    dir: Option<String>,
    kind: Option<String>,
) -> Result<BackupRow, String> {
    let live = db::db_path(&app)?;
    let pool = db::connect(&live).await?;
    let target_dir = match dir.filter(|d| !d.trim().is_empty()) {
        Some(d) => PathBuf::from(d.trim()),
        None => app
            .path()
            .app_config_dir()
            .map_err(|e| e.to_string())?
            .join("backups"),
    };
    let kind = match kind.as_deref() {
        Some("auto") => "auto",
        _ => "manual",
    };
    let row = backup_to_file(&pool, &target_dir, kind).await?;
    if kind != "pre_restore" {
        let keep = retention_setting(&pool).await?;
        let _ = prune_backups(&pool, keep).await;
    }
    pool.close().await;
    Ok(row)
}

#[tauri::command]
pub async fn list_backups<R: Runtime>(app: AppHandle<R>) -> Result<Vec<BackupRow>, String> {
    let pool = db::pool(&app).await?;
    ensure_backups_table(&pool).await?;
    let rows: Vec<(i64, String, i64, String, String)> = sqlx::query_as(
        "SELECT id, file_path, size_bytes, status, created_at FROM backups
         ORDER BY datetime(created_at) DESC, id DESC",
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| e.to_string())?;
    pool.close().await;
    Ok(rows
        .into_iter()
        .map(|r| BackupRow {
            id: r.0,
            path: r.1,
            size_bytes: r.2,
            kind: r.3,
            created_at: r.4,
        })
        .collect())
}

/// Removes one backup entry and its managed file.
#[tauri::command]
pub async fn delete_backup<R: Runtime>(app: AppHandle<R>, path: String) -> Result<(), String> {
    let p = PathBuf::from(&path);
    if !managed_backup_file(&p) {
        return Err("Only files created by the app's backups can be deleted here".to_string());
    }
    let live = db::db_path(&app)?;
    let pool = db::connect(&live).await?;
    ensure_backups_table(&pool).await?;
    sqlx::query("DELETE FROM backups WHERE file_path = ?")
        .bind(&path)
        .execute(&pool)
        .await
        .map_err(|e| e.to_string())?;
    pool.close().await;
    match fs::remove_file(&p) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

/// F8.1 — on-demand integrity check for the Settings screen.
#[tauri::command]
pub async fn check_db_integrity<R: Runtime>(app: AppHandle<R>) -> Result<String, String> {
    let path = db::db_path(&app)?;
    db::integrity_check(&path).await
}

/// F8.5 — replace the current database with the chosen backup file.
/// The frontend must close its SQL connections before invoking this.
#[tauri::command]
pub async fn restore_database<R: Runtime>(
    app: AppHandle<R>,
    source_path: String,
) -> Result<BackupRow, String> {
    let live = db::db_path(&app)?;
    let source = PathBuf::from(&source_path);
    if !source.exists() {
        return Err("Backup file not found".to_string());
    }
    let safety_dir = app
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?
        .join("backups");
    restore_from_file(&live, &source, &safety_dir).await
}

/// F8.4 — dumps every business table into a multi-sheet `.xlsx` workbook.
#[tauri::command]
pub async fn export_full_workbook<R: Runtime>(
    app: AppHandle<R>,
    path: String,
) -> Result<usize, String> {
    let pool = db::pool(&app).await?;

    async fn dump_table(pool: &SqlitePool, table: &str, sheet: &str) -> Result<SheetData, String> {
        let rows = sqlx::query(&format!("SELECT * FROM \"{table}\""))
            .fetch_all(pool)
            .await
            .map_err(|e| e.to_string())?;
        let headers: Vec<String> = rows
            .first()
            .map(|r| {
                r.columns()
                    .iter()
                    .map(|c| c.name().to_string())
                    .collect()
            })
            .unwrap_or_default();
        let mut data = Vec::with_capacity(rows.len());
        for row in &rows {
            data.push((0..row.columns().len()).map(|i| cell_to_string(row, i)).collect());
        }
        Ok(SheetData {
            name: sheet.to_string(),
            headers,
            rows: data,
        })
    }

    let sheets = vec![
        dump_table(&pool, "products", "Products").await?,
        dump_table(&pool, "categories", "Categories").await?,
        dump_table(&pool, "suppliers", "Suppliers").await?,
        dump_table(&pool, "supplier_invoices", "SupplierInvoices").await?,
        dump_table(&pool, "supplier_invoice_items", "SupplierInvoiceItems").await?,
        dump_table(&pool, "customers", "Customers").await?,
        dump_table(&pool, "customer_ledger", "CustomerLedger").await?,
        dump_table(&pool, "sales", "Sales").await?,
        dump_table(&pool, "sale_items", "SaleItems").await?,
        dump_table(&pool, "payments", "Payments").await?,
        dump_table(&pool, "expense_out", "Expenses").await?,
        dump_table(&pool, "stock_movements", "StockMovements").await?,
    ];
    let count = sheets.len();
    pool.close().await;

    export::write_xlsx_multi(Path::new(&path), &sheets)?;
    Ok(count)
}

/// Stringifies any SQLite value for the workbook export.
fn cell_to_string(row: &sqlx::sqlite::SqliteRow, i: usize) -> String {
    if let Ok(Some(v)) = row.try_get::<Option<i64>, _>(i) {
        return v.to_string();
    }
    if let Ok(Some(v)) = row.try_get::<Option<f64>, _>(i) {
        return format!("{v}");
    }
    if let Ok(Some(v)) = row.try_get::<Option<String>, _>(i) {
        return v;
    }
    if let Ok(Some(v)) = row.try_get::<Option<Vec<u8>>, _>(i) {
        return format!("<blob {} B>", v.len());
    }
    String::new()
}

/* ------------------------------------------------------------------ */
/* Tests                                                               */
/* ------------------------------------------------------------------ */

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use std::sync::atomic::AtomicUsize;

    static SEQ: AtomicUsize = AtomicUsize::new(0);

    fn unique_tmp(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "pos8_{tag}_{}_{}.db",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::SeqCst)
        ))
    }

    async fn make_db(path: &Path, marker: &str) {
        let pool = db::connect(path).await.unwrap();
        sqlx::query("CREATE TABLE IF NOT EXISTS demo (id INTEGER PRIMARY KEY AUTOINCREMENT, tag TEXT NOT NULL)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO demo (tag) VALUES (?)")
            .bind(marker)
            .execute(&pool)
            .await
            .unwrap();
        pool.close().await;
    }

    async fn count_demo(path: &Path, tag: &str) -> i64 {
        let pool = db::connect(path).await.unwrap();
        let n: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM demo WHERE tag = ?")
            .bind(tag)
            .fetch_one(&pool)
            .await
            .unwrap();
        pool.close().await;
        n.0
    }

    #[tokio::test]
    async fn backup_creates_snapshot_and_row() {
        let src = unique_tmp("src");
        let dir = unique_tmp("dir");
        let _ = fs::remove_file(&src);
        make_db(&src, "alpha").await;

        let pool = db::connect(&src).await.unwrap();
        let row = backup_to_file(&pool, &dir, "manual").await.unwrap();
        pool.close().await;

        assert!(PathBuf::from(&row.path).exists());
        assert!(row.size_bytes > 0);
        assert_eq!(row.kind, "manual");
        assert_eq!(count_demo(Path::new(&row.path), "alpha").await, 1);

        let _ = fs::remove_file(&src);
        let _ = fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn prune_keeps_last_n_and_removes_files() {
        let scratch = unique_tmp("scratch");
        let pool = db::connect(&scratch).await.unwrap();
        ensure_backups_table(&pool).await.unwrap();

        let mut paths = Vec::new();
        for i in 0..4 {
            let p = unique_tmp(&format!("bk{i}"));
            fs::write(&p, b"x").unwrap();
            sqlx::query("INSERT INTO backups (file_path, size_bytes, status) VALUES (?, 1, 'manual')")
                .bind(p.to_string_lossy().to_string())
                .execute(&pool)
                .await
                .unwrap();
            paths.push(p);
        }

        let pruned = prune_backups(&pool, 2).await.unwrap();
        pool.close().await;

        assert_eq!(pruned, 2);
        assert!(!paths[0].exists());
        assert!(!paths[1].exists());
        assert!(paths[2].exists());
        assert!(paths[3].exists());

        let _ = fs::remove_file(&scratch);
        for p in &paths[2..] {
            let _ = fs::remove_file(p);
        }
    }

    #[tokio::test]
    async fn restore_replaces_database_and_keeps_safety_copy() {
        let good = unique_tmp("good");
        let live = unique_tmp("live");
        let dir = unique_tmp("rdir");
        let _ = fs::remove_file(&good);
        let _ = fs::remove_file(&live);
        make_db(&good, "wanted").await;
        make_db(&live, "current").await;

        // Snapshot the "good" state.
        {
            let pool = db::connect(&good).await.unwrap();
            let _row = backup_to_file(&pool, &dir, "manual").await.unwrap();
            pool.close().await;

            // Live currently holds "current"; restore brings back "wanted".
            let safety = restore_from_file(&live, &good, &dir).await.unwrap();
            assert!(PathBuf::from(&safety.path).exists());
            assert_eq!(safety.kind, "pre_restore");
        }

        assert_eq!(count_demo(&live, "wanted").await, 1);
        assert_eq!(count_demo(&live, "current").await, 0);

        let _ = fs::remove_file(&good);
        let _ = fs::remove_file(&live);
        let _ = fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn restore_rejects_non_sqlite_files() {
        let fake = unique_tmp("fake");
        fs::write(&fake, b"definitely not a database").unwrap();
        let live = unique_tmp("live2");
        let dir = unique_tmp("rdir2");

        let err = restore_from_file(&live, &fake, &dir).await.unwrap_err();
        assert!(err.contains("not a SQLite database"), "got: {err}");

        let _ = fs::remove_file(&fake);
        let _ = fs::remove_file(&live);
        let _ = fs::remove_dir_all(&dir);
    }
}
