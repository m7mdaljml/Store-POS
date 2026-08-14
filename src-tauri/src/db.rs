use std::path::{Path, PathBuf};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
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
        .create_if_missing(true);
    SqlitePoolOptions::new()
        .connect_with(opts)
        .await
        .map_err(|e| e.to_string())
}
