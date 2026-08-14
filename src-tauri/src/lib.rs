use tauri_plugin_sql::{Migration, MigrationKind};

mod commands;
mod db;
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
      commands::auth::update_user_permissions
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
