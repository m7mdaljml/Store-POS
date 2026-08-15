use tauri_plugin_sql::{Migration, MigrationKind};

mod commands;
mod db;
mod export;
mod seed;

fn initial_migrations() -> Vec<Migration> {
  vec![Migration {
    version: 1,
    description: "create_initial_tables",
    sql: include_str!("../migrations/001_initial.sql"),
    kind: MigrationKind::Up,
  }]
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
      commands::expenses::export_expenses
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
