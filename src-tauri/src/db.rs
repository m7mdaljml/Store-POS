use std::path::{Path, PathBuf};
use std::time::Duration;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions};
use tauri::{AppHandle, Manager, Runtime};

pub fn db_path<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|dir| dir.join("store.db"))
        .map_err(|e| e.to_string())
}

pub async fn pool<R: Runtime>(app: &AppHandle<R>) -> Result<SqlitePool, String> {
    let path = db_path(app)?;
    connect(&path).await
}

pub async fn connect(path: &Path) -> Result<SqlitePool, String> {
    let opts = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        // F8.1: WAL improves crash safety and allows concurrent readers. The
        // setting is persistent — once written to the file it also applies to
        // connections opened by the frontend's plugin-sql pool. Note: the very
        // first conversion of a rollback-journal file needs a brief exclusive
        // lock, hence the generous busy timeout below.
        .journal_mode(SqliteJournalMode::Wal)
        // Wait instead of failing when another pool (frontend or a backup
        // command) holds the write lock.
        .busy_timeout(Duration::from_secs(10));
    SqlitePoolOptions::new()
        .connect_with(opts)
        .await
        .map_err(|e| e.to_string())
}

/// Runs `PRAGMA integrity_check` against the database file (F8.1).
/// Returns "ok" when healthy, otherwise a semicolon-joined issue list.
pub async fn integrity_check(path: &Path) -> Result<String, String> {
    let pool = connect(path).await?;
    let rows: Vec<(String,)> = sqlx::query_as("PRAGMA integrity_check")
        .fetch_all(&pool)
        .await
        .map_err(|e| e.to_string())?;
    pool.close().await;
    Ok(rows
        .into_iter()
        .map(|r| r.0)
        .collect::<Vec<_>>()
        .join("; "))
}
