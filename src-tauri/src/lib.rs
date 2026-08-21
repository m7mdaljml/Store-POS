use tauri_plugin_sql::{Migration, MigrationKind};

mod commands;
mod db;
mod export;
mod seed;

fn initial_migrations() -> Vec<Migration> {
  vec![
    Migration {
      version: 1,
      description: "create_initial_tables",
      sql: include_str!("../migrations/001_initial.sql"),
      kind: MigrationKind::Up,
    },
    Migration {
      version: 2,
      description: "sale_sessions_variance",
      sql: "ALTER TABLE sale_sessions ADD COLUMN variance REAL;",
      kind: MigrationKind::Up,
    },
    Migration {
      version: 3,
      description: "sale_sessions_closed_by",
      sql: "ALTER TABLE sale_sessions ADD COLUMN closed_by INTEGER REFERENCES users(id);",
      kind: MigrationKind::Up,
    },
    Migration {
      version: 4,
      description: "sale_sessions_notes",
      sql: "ALTER TABLE sale_sessions ADD COLUMN notes TEXT;",
      kind: MigrationKind::Up,
    },
    // Applied releases must never drop entries from this list. v5 is kept for
    // machines where it already ran; it is a no-op because `backups` ships
    // with the initial schema (001_initial.sql) using file_path/status.
    Migration {
      version: 5,
      description: "backups_table",
      sql: "CREATE TABLE IF NOT EXISTS backups (\n  id         INTEGER PRIMARY KEY AUTOINCREMENT,\n  path       TEXT UNIQUE NOT NULL,\n  size_bytes INTEGER NOT NULL DEFAULT 0,\n  kind       TEXT NOT NULL DEFAULT 'manual',\n  created_at TEXT NOT NULL DEFAULT (datetime('now'))\n);",
      kind: MigrationKind::Up,
    },
  ]
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_dialog::init())
    .plugin(
      tauri_plugin_sql::Builder::default()
        .add_migrations("sqlite:store.db", initial_migrations())
        .build(),
    )
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }
      let handle = app.handle().clone();
      tauri::async_runtime::spawn(async move {
        match seed::seed_app(&handle).await {
          Ok(()) => log::info!("Seed data ready"),
          Err(e) => log::error!("Seeding failed: {e}"),
        }
        // F8.1: verify database health once at startup.
        match db::db_path(&handle) {
          Ok(path) => match db::integrity_check(&path).await {
            Ok(result) => log::info!("DB integrity check: {result}"),
            Err(e) => log::error!("DB integrity check failed: {e}"),
          },
          Err(e) => log::error!("Cannot resolve DB path for integrity check: {e}"),
        }
      });
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      commands::auth::hash_password,
      commands::auth::verify_password,
      commands::auth::login,
      commands::auth::logout,
      commands::auth::verify_session,
      commands::auth::create_user,
      commands::auth::delete_user,
      commands::auth::set_user_active,
      commands::auth::remove_user,
      commands::auth::update_user,
      commands::auth::update_user_permissions,
      commands::auth::list_roles,
      commands::auth::list_permissions,
      commands::auth::list_users,
      commands::catalog::list_categories,
      commands::catalog::create_category,
      commands::catalog::update_category,
      commands::catalog::delete_category,
      commands::catalog::create_product,
      commands::catalog::update_product,
      commands::catalog::delete_product,
      commands::catalog::set_product_active,
      commands::catalog::adjust_stock,
      commands::catalog::import_product_image,
      commands::catalog::import_products_csv,
      commands::catalog::list_stock_movements,
      commands::suppliers::list_suppliers,
      commands::suppliers::get_supplier,
      commands::suppliers::create_supplier,
      commands::suppliers::update_supplier,
      commands::suppliers::delete_supplier,
      commands::customers::record_customer_payment,
      commands::reports::sales_summary,
      commands::reports::revenue_trend,
      commands::reports::top_products,
      commands::reports::category_breakdown,
      commands::reports::sales_report,
      commands::reports::inventory_report,
      commands::reports::margin_report,
      commands::reports::export_sales_report,
      commands::reports::export_inventory,
      commands::reports::export_top_products,
      commands::backup::create_backup,
      commands::backup::list_backups,
      commands::backup::delete_backup,
      commands::backup::check_db_integrity,
      commands::backup::restore_database,
      commands::backup::export_full_workbook,
      commands::purchasing::create_supplier_invoice,
      commands::purchasing::list_supplier_invoices,
      commands::purchasing::add_supplier_payment,
      commands::purchasing::list_supplier_payments,
      commands::expenses::list_expense_categories,
      commands::expenses::create_expense_category,
      commands::expenses::delete_expense_category,
      commands::expenses::list_expenses_out,
      commands::expenses::add_expense_out,
      commands::expenses::list_expenses,
      commands::expenses::expense_summary,
      commands::expenses::export_expenses,
      commands::sales::create_sale,
      commands::sales::void_sale,
      commands::sales::list_sales,
      commands::sales::hold_sale,
      commands::sales::resume_sale,
      commands::sales::cancel_held_sale,
      commands::sales::get_sale_receipt,
      commands::sessions::open_session,
      commands::sessions::close_session,
      commands::sessions::get_open_session,
      commands::sessions::list_sessions
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
