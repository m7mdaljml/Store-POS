//! F9.6 E2E smoke flow: seed a fresh database, log a cashier in, stock a
//! product, ring up a sale with payment, verify the receipt data, pull the
//! daily report, and take a backup snapshot — the full happy path a real
//! session would exercise.

use app_lib::{commands, db, initial_migrations, seed};

fn temp_path(tag: &str) -> std::path::PathBuf {
    static SEQ: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    std::env::temp_dir().join(format!("smoke_{tag}_{}_{}.db", std::process::id(), n))
}

#[tokio::test]
async fn full_sale_flow_smoke() {
    let path = temp_path("flow");
    let _ = std::fs::remove_file(&path);

    // 1. Fresh install → apply migrations (same as the runtime plugin) then
    //    seed the demo/reference data.
    db::apply_migrations(&path, &initial_migrations()).await.expect("migrations failed");
    seed::seed_db(&path).await.expect("seed failed");
    let pool = db::connect(&path).await.unwrap();

    // 2. Login: hash a password, store a cashier, verify it matches.
    let hash = commands::auth::hash_password("cashier-pass".into()).unwrap();
    let role_id: i64 = sqlx::query_scalar(
        "SELECT id FROM roles WHERE name = 'cashier'
         UNION ALL SELECT id FROM roles LIMIT 1",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO users (username, password_hash, full_name, role_id, is_active)
         VALUES ('smoke.cashier', ?, 'Smoke Cashier', ?, 1)",
    )
    .bind(&hash)
    .bind(role_id)
    .execute(&pool)
    .await
    .unwrap();
    assert!(commands::auth::verify_password("cashier-pass".into(), hash).unwrap());
    let user_id: i64 =
        sqlx::query_scalar("SELECT id FROM users WHERE username = 'smoke.cashier'")
            .fetch_one(&pool)
            .await
            .unwrap();

    // Baseline for report deltas (seed may add its own demo sales later).
    let baseline = commands::reports::compute_summary(&pool, "2000-01-01", "2099-12-31")
        .await
        .unwrap();

    // 3. Stock a product.
    sqlx::query(
        "INSERT INTO products (sku, name, cost_price, sell_price, unit, stock_qty)
         VALUES ('SMOKE-1', 'Smoke Widget', 4.0, 9.5, 'pc', 50)",
    )
    .execute(&pool)
    .await
    .unwrap();
    let product_id: i64 = sqlx::query_scalar("SELECT id FROM products WHERE sku = 'SMOKE-1'")
        .fetch_one(&pool)
        .await
        .unwrap();

    // 4. Ring up a sale: 2 widgets @ 9.5 = 19.0 paid in cash (change 1.0).
    sqlx::query(
        "INSERT INTO sales (sale_no, user_id, subtotal, discount, tax, total, paid_amount, change_given, status)
         VALUES ('S-SMOKE-1', ?, 19.0, 0, 0, 19.0, 20.0, 1.0, 'completed')",
    )
    .bind(user_id)
    .execute(&pool)
    .await
    .unwrap();
    let sale_id: i64 = sqlx::query_scalar("SELECT id FROM sales WHERE sale_no = 'S-SMOKE-1'")
        .fetch_one(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO sale_items (sale_id, product_id, qty, price, cost_price, discount, tax, subtotal)
         VALUES (?, ?, 2, 9.5, 4.0, 0, 0, 19.0)",
    )
    .bind(sale_id)
    .bind(product_id)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO payments (sale_id, method, amount) VALUES (?, 'cash', 20.0)")
        .bind(sale_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE products SET stock_qty = stock_qty - 2 WHERE id = ?")
        .bind(product_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO stock_movements (product_id, type, qty, notes, user_id)
         VALUES (?, 'sale', -2, 'smoke sale', ?)",
    )
    .bind(product_id)
    .bind(user_id)
    .execute(&pool)
    .await
    .unwrap();

    // 5. Receipt data must line up (header comes from settings).
    sqlx::query("INSERT INTO settings (key, value) VALUES ('store_name', 'Smoke Store')")
        .execute(&pool)
        .await
        .unwrap();
    let (store_name, total, paid): (String, f64, f64) = sqlx::query_as(
        "SELECT (SELECT value FROM settings WHERE key = 'store_name'), s.total, s.paid_amount
         FROM sales s WHERE s.id = ?",
    )
    .bind(sale_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(store_name, "Smoke Store");
    assert!((total - 19.0).abs() < 0.001);
    assert!((paid - 20.0).abs() < 0.001);

    // 6. Daily report reflects exactly one more order worth 19.0.
    let summary = commands::reports::compute_summary(&pool, "2000-01-01", "2099-12-31")
        .await
        .unwrap();
    assert_eq!(summary.orders, baseline.orders + 1);
    assert!((summary.revenue - baseline.revenue - 19.0).abs() < 0.001);

    // 7. Backup snapshot succeeds against the live-looking database.
    let dir = temp_path("dir");
    let row = commands::backup::backup_to_file(&pool, &dir, "manual")
        .await
        .unwrap();
    assert!(std::path::Path::new(&row.path).is_file());
    assert!(row.size_bytes > 0);

    pool.close().await;
    let _ = std::fs::remove_file(&path);
}
