use serde::{Deserialize, Serialize};
use sqlx::Row;
use tauri::{AppHandle, Manager, Runtime};

use super::Page;
use crate::db;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryRecord {
    pub id: i64,
    pub name: String,
    pub parent_id: Option<i64>,
    pub product_count: i64,
}

#[tauri::command]
pub async fn list_categories<R: Runtime>(
    app: AppHandle<R>,
) -> Result<Vec<CategoryRecord>, String> {
    let pool = db::pool(&app).await?;
    let rows = sqlx::query(
        "SELECT c.id, c.name, c.parent_id, COUNT(p.id) AS product_count
         FROM categories c
         LEFT JOIN products p ON p.category_id = c.id
         GROUP BY c.id
         ORDER BY c.name",
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| e.to_string())?;
    rows.into_iter()
        .map(|row| {
            Ok(CategoryRecord {
                id: row.try_get("id").map_err(|e| e.to_string())?,
                name: row.try_get("name").map_err(|e| e.to_string())?,
                parent_id: row.try_get("parent_id").map_err(|e| e.to_string())?,
                product_count: row.try_get("product_count").map_err(|e| e.to_string())?,
            })
        })
        .collect()
}

#[tauri::command]
pub async fn create_category<R: Runtime>(
    app: AppHandle<R>,
    name: String,
    parent_id: Option<i64>,
    user_id: Option<i64>,
) -> Result<i64, String> {
    let pool = db::pool(&app).await?;
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("Category name is required".into());
    }
    let exists: Option<(i64,)> = sqlx::query_as("SELECT id FROM categories WHERE name = ?")
        .bind(&name)
        .fetch_optional(&pool)
        .await
        .map_err(|e| e.to_string())?;
    if exists.is_some() {
        return Err(format!("Category '{name}' already exists"));
    }
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    let result = sqlx::query("INSERT INTO categories (name, parent_id) VALUES (?, ?)")
        .bind(&name)
        .bind(parent_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    let id = result.last_insert_rowid();
    sqlx::query(
        "INSERT INTO audit_log (user_id, action, entity_type, entity_id, details)
         VALUES (?, 'category.create', 'category', ?, ?)",
    )
    .bind(user_id)
    .bind(id)
    .bind(format!("Created category \"{name}\""))
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
pub async fn update_category<R: Runtime>(
    app: AppHandle<R>,
    category_id: i64,
    name: String,
    parent_id: Option<i64>,
    user_id: Option<i64>,
) -> Result<(), String> {
    let pool = db::pool(&app).await?;
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("Category name is required".into());
    }
    if parent_id == Some(category_id) {
        return Err("A category cannot be its own parent".into());
    }
    let exists: Option<(i64,)> =
        sqlx::query_as("SELECT id FROM categories WHERE name = ? AND id != ?")
            .bind(&name)
            .bind(category_id)
            .fetch_optional(&pool)
            .await
            .map_err(|e| e.to_string())?;
    if exists.is_some() {
        return Err(format!("Category '{name}' already exists"));
    }
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    let result = sqlx::query("UPDATE categories SET name = ?, parent_id = ? WHERE id = ?")
        .bind(&name)
        .bind(parent_id)
        .bind(category_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    if result.rows_affected() == 0 {
        return Err("Category not found".into());
    }
    sqlx::query(
        "INSERT INTO audit_log (user_id, action, entity_type, entity_id, details)
         VALUES (?, 'category.update', 'category', ?, ?)",
    )
    .bind(user_id)
    .bind(category_id)
    .bind(format!("Updated category \"{name}\""))
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn delete_category<R: Runtime>(
    app: AppHandle<R>,
    category_id: i64,
    user_id: Option<i64>,
) -> Result<(), String> {
    let pool = db::pool(&app).await?;
    let products: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM products WHERE category_id = ?")
        .bind(category_id)
        .fetch_one(&pool)
        .await
        .map_err(|e| e.to_string())?;
    if products.0 > 0 {
        return Err(
            "This category still has products. Move or delete them before removing the category."
                .into(),
        );
    }
    let sub: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM categories WHERE parent_id = ?")
        .bind(category_id)
        .fetch_one(&pool)
        .await
        .map_err(|e| e.to_string())?;
    if sub.0 > 0 {
        return Err("This category has sub-categories. Remove them first.".into());
    }
    let name_row: Option<(String,)> = sqlx::query_as("SELECT name FROM categories WHERE id = ?")
        .bind(category_id)
        .fetch_optional(&pool)
        .await
        .map_err(|e| e.to_string())?;
    let cat_name = name_row.map(|r| r.0).unwrap_or_default();
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    let result = sqlx::query("DELETE FROM categories WHERE id = ?")
        .bind(category_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    if result.rows_affected() == 0 {
        return Err("Category not found".into());
    }
    sqlx::query(
        "INSERT INTO audit_log (user_id, action, entity_type, entity_id, details)
         VALUES (?, 'category.delete', 'category', ?, ?)",
    )
    .bind(user_id)
    .bind(category_id)
    .bind(format!("Deleted category \"{cat_name}\""))
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductInput {
    pub sku: Option<String>,
    pub barcode: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub category_id: Option<i64>,
    pub cost_price: f64,
    pub sell_price: f64,
    pub tax_profile_id: Option<i64>,
    pub unit: String,
    pub reorder_level: f64,
    pub image_path: Option<String>,
    pub is_active: bool,
    #[serde(default)]
    pub is_quick: bool,
    #[serde(default)]
    pub opening_stock: f64,
}

fn validate_product(input: &ProductInput) -> Result<(), String> {
    if input.name.trim().is_empty() {
        return Err("Product name is required".into());
    }
    let barcode = input.barcode.as_ref().map(|s| s.trim());
    if barcode.map_or(true, |s| s.is_empty()) {
        return Err("Barcode is required".into());
    }
    if input.cost_price < 0.0 {
        return Err("Cost price cannot be negative".into());
    }
    if input.sell_price < 0.0 {
        return Err("Sell price cannot be negative".into());
    }
    if input.sell_price <= input.cost_price {
        return Err("Sell price must be more than cost price".into());
    }
    if input.unit.trim().is_empty() {
        return Err("Unit is required".into());
    }
    if input.opening_stock < 0.0 {
        return Err("Opening stock cannot be negative".into());
    }
    Ok(())
}

async fn ensure_sku_unique(
    pool: &sqlx::SqlitePool,
    sku: &Option<String>,
    product_id: Option<i64>,
) -> Result<(), String> {
    let Some(sku) = sku.as_ref().map(|s| s.trim()) else {
        return Ok(());
    };
    if sku.is_empty() {
        return Ok(());
    }
    let taken: Option<(i64,)> = match product_id {
        Some(id) => {
            sqlx::query_as("SELECT id FROM products WHERE sku = ? AND id != ?")
                .bind(sku)
                .bind(id)
                .fetch_optional(pool)
                .await
                .map_err(|e| e.to_string())?
        }
        None => {
            sqlx::query_as("SELECT id FROM products WHERE sku = ?")
                .bind(sku)
                .fetch_optional(pool)
                .await
                .map_err(|e| e.to_string())?
        }
    };
    if taken.is_some() {
        return Err(format!("SKU '{sku}' already exists"));
    }
    Ok(())
}

fn empty_to_none(value: &Option<String>) -> Option<String> {
    value.as_ref().map(|v| v.trim()).filter(|v| !v.is_empty()).map(String::from)
}

#[tauri::command]
pub async fn create_product<R: Runtime>(
    app: AppHandle<R>,
    input: ProductInput,
    user_id: Option<i64>,
) -> Result<i64, String> {
    let pool = db::pool(&app).await?;
    validate_product(&input)?;
    ensure_sku_unique(&pool, &input.sku, None).await?;

    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    let result = sqlx::query(
        "INSERT INTO products
            (sku, barcode, name, description, category_id, cost_price, sell_price,
             tax_profile_id, unit, stock_qty, reorder_level, image_path, is_active, is_quick)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(empty_to_none(&input.sku))
    .bind(empty_to_none(&input.barcode))
    .bind(input.name.trim())
    .bind(empty_to_none(&input.description))
    .bind(input.category_id)
    .bind(input.cost_price)
    .bind(input.sell_price)
    .bind(input.tax_profile_id)
    .bind(input.unit.trim())
    .bind(input.opening_stock)
    .bind(input.reorder_level)
    .bind(empty_to_none(&input.image_path))
    .bind(i64::from(input.is_active))
    .bind(i64::from(input.is_quick))
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    let id = result.last_insert_rowid();
    if input.opening_stock > 0.0 {
        sqlx::query(
            "INSERT INTO stock_movements (product_id, type, qty, notes)
             VALUES (?, 'opening', ?, 'Opening stock set on create')",
        )
        .bind(id)
        .bind(input.opening_stock)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    }
    sqlx::query(
        "INSERT INTO audit_log (user_id, action, entity_type, entity_id, details)
         VALUES (?, 'product.create', 'product', ?, ?)",
    )
    .bind(user_id)
    .bind(id)
    .bind(format!("Created product \"{}\"", input.name.trim()))
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
pub async fn update_product<R: Runtime>(
    app: AppHandle<R>,
    product_id: i64,
    input: ProductInput,
    user_id: Option<i64>,
) -> Result<(), String> {
    let pool = db::pool(&app).await?;
    validate_product(&input)?;
    ensure_sku_unique(&pool, &input.sku, Some(product_id)).await?;

    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    let result = sqlx::query(
        "UPDATE products SET
            sku = ?, barcode = ?, name = ?, description = ?, category_id = ?,
            cost_price = ?, sell_price = ?, tax_profile_id = ?, unit = ?,
            reorder_level = ?, image_path = ?, is_active = ?, is_quick = ?
         WHERE id = ?",
    )
    .bind(empty_to_none(&input.sku))
    .bind(empty_to_none(&input.barcode))
    .bind(input.name.trim())
    .bind(empty_to_none(&input.description))
    .bind(input.category_id)
    .bind(input.cost_price)
    .bind(input.sell_price)
    .bind(input.tax_profile_id)
    .bind(input.unit.trim())
    .bind(input.reorder_level)
    .bind(empty_to_none(&input.image_path))
    .bind(i64::from(input.is_active))
    .bind(i64::from(input.is_quick))
    .bind(product_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    if result.rows_affected() == 0 {
        return Err("Product not found".into());
    }
    sqlx::query(
        "INSERT INTO audit_log (user_id, action, entity_type, entity_id, details)
         VALUES (?, 'product.update', 'product', ?, ?)",
    )
    .bind(user_id)
    .bind(product_id)
    .bind(format!("Updated product \"{}\"", input.name.trim()))
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn set_product_active<R: Runtime>(
    app: AppHandle<R>,
    product_id: i64,
    is_active: bool,
    user_id: Option<i64>,
) -> Result<(), String> {
    let pool = db::pool(&app).await?;
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    let result = sqlx::query("UPDATE products SET is_active = ? WHERE id = ?")
        .bind(i64::from(is_active))
        .bind(product_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    if result.rows_affected() == 0 {
        return Err("Product not found".into());
    }
    let action = if is_active { "product.activate" } else { "product.deactivate" };
    let detail = if is_active {
        format!("Activated product {product_id}")
    } else {
        format!("Deactivated product {product_id}")
    };
    sqlx::query(
        "INSERT INTO audit_log (user_id, action, entity_type, entity_id, details)
         VALUES (?, ?, 'product', ?, ?)",
    )
    .bind(user_id)
    .bind(action)
    .bind(product_id)
    .bind(detail)
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn delete_product<R: Runtime>(app: AppHandle<R>, product_id: i64, user_id: Option<i64>) -> Result<(), String> {
    let pool = db::pool(&app).await?;
    let sold: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM sale_items WHERE product_id = ?")
        .bind(product_id)
        .fetch_one(&pool)
        .await
        .map_err(|e| e.to_string())?;
    if sold.0 > 0 {
        return Err(
            "This product has sale history and cannot be deleted. Deactivate it in the edit form instead."
                .into(),
        );
    }
    let name_row: Option<(String,)> = sqlx::query_as("SELECT name FROM products WHERE id = ?")
        .bind(product_id)
        .fetch_optional(&pool)
        .await
        .map_err(|e| e.to_string())?;
    let prod_name = name_row.map(|r| r.0).unwrap_or_default();
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    let result = sqlx::query("DELETE FROM products WHERE id = ?")
        .bind(product_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    if result.rows_affected() == 0 {
        return Err("Product not found".into());
    }
    sqlx::query(
        "INSERT INTO audit_log (user_id, action, entity_type, entity_id, details)
         VALUES (?, 'product.delete', 'product', ?, ?)",
    )
    .bind(user_id)
    .bind(product_id)
    .bind(format!("Deleted product \"{prod_name}\""))
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn import_product_image<R: Runtime>(
    app: AppHandle<R>,
    product_id: i64,
    source_path: String,
) -> Result<String, String> {
    let source = std::path::Path::new(&source_path);
    if !source.is_file() {
        return Err("Selected file does not exist".into());
    }

    let images_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("images");
    std::fs::create_dir_all(&images_dir).map_err(|e| e.to_string())?;

    let ext = source
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_else(|| "png".into());
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or_default();
    let dest = images_dir.join(format!("product_{product_id}_{stamp}.{ext}"));

    std::fs::copy(source, &dest).map_err(|e| format!("Could not copy image: {e}"))?;
    let dest_path = dest.to_string_lossy().to_string();

    let pool = db::pool(&app).await?;
    sqlx::query("UPDATE products SET image_path = ? WHERE id = ?")
        .bind(&dest_path)
        .bind(product_id)
        .execute(&pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(dest_path)
}

#[tauri::command]
pub async fn adjust_stock<R: Runtime>(
    app: AppHandle<R>,
    product_id: i64,
    qty: f64,
    notes: Option<String>,
    user_id: Option<i64>,
) -> Result<(), String> {
    let pool = db::pool(&app).await?;
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    let current: Option<(f64,)> = sqlx::query_as("SELECT stock_qty FROM products WHERE id = ?")
        .bind(product_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    let current = match current {
        Some((v,)) => v,
        None => return Err("Product not found".into()),
    };

    let new_qty = current + qty;
    if new_qty < 0.0 {
        return Err("Stock cannot go below zero".into());
    }

    sqlx::query(
        "INSERT INTO stock_movements (product_id, type, qty, notes, user_id) VALUES (?, 'adjustment', ?, ?, ?)",
    )
    .bind(product_id)
    .bind(qty)
    .bind(notes)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query("UPDATE products SET stock_qty = ? WHERE id = ?")
        .bind(new_qty)
        .bind(product_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    sqlx::query(
        "INSERT INTO audit_log (user_id, action, entity_type, entity_id, details)
         VALUES (?, 'stock.adjust', 'product', ?, ?)",
    )
    .bind(user_id)
    .bind(product_id)
    .bind(format!("Stock adjusted from {current} to {new_qty}"))
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StockMovement {
    pub id: i64,
    pub product_id: i64,
    pub product_name: String,
    pub movement_type: String,
    pub qty: f64,
    pub notes: Option<String>,
    pub user_name: Option<String>,
    pub created_at: String,
}

#[tauri::command]
pub async fn list_stock_movements<R: Runtime>(
    app: AppHandle<R>,
    movement_type: Option<String>,
    product_id: Option<i64>,
    search: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Page<StockMovement>, String> {
    let pool = db::pool(&app).await?;

    let pattern = search
        .as_deref()
        .map(|s| format!("%{}%", s.trim()))
        .filter(|p| p != "%%");
    let mut conds: Vec<&str> = Vec::new();
    if movement_type.is_some() {
        conds.push("sm.type = ?");
    }
    if product_id.is_some() {
        conds.push("sm.product_id = ?");
    }
    if pattern.is_some() {
        conds.push("(p.name LIKE ? OR COALESCE(sm.notes, '') LIKE ? OR COALESCE(u.full_name, '') LIKE ?)");
    }
    let where_clause = if conds.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", conds.join(" AND "))
    };
    let sql = format!(
        "SELECT sm.id, sm.product_id, p.name, sm.type, sm.qty, sm.notes, u.full_name, sm.created_at
         FROM stock_movements sm
         JOIN products p ON p.id = sm.product_id
         LEFT JOIN users u ON u.id = sm.user_id
         {where_clause}
         ORDER BY sm.created_at DESC, sm.id DESC
         LIMIT ? OFFSET ?"
    );

    let mut query = sqlx::query(&sql);
    if let Some(t) = &movement_type {
        query = query.bind(t);
    }
    if let Some(pid) = &product_id {
        query = query.bind(pid);
    }
    if let Some(p) = &pattern {
        query = query.bind(p).bind(p).bind(p);
    }
    let rows = query
        .bind(limit.map(|l| l.max(1)).unwrap_or(-1))
        .bind(offset.unwrap_or(0).max(0))
        .fetch_all(&pool)
        .await
        .map_err(|e| e.to_string())?;

    let items: Vec<StockMovement> = rows
        .into_iter()
        .map(|r| StockMovement {
            id: r.get(0),
            product_id: r.get(1),
            product_name: r.get(2),
            movement_type: r.get(3),
            qty: r.get(4),
            notes: r.get(5),
            user_name: r.get(6),
            created_at: r.get(7),
        })
        .collect();

    let count_sql = format!(
        "SELECT COUNT(*) FROM stock_movements sm
         JOIN products p ON p.id = sm.product_id
         LEFT JOIN users u ON u.id = sm.user_id
         {where_clause}"
    );
    let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
    if movement_type.is_some() {
        count_query = count_query.bind(&movement_type);
    }
    if product_id.is_some() {
        count_query = count_query.bind(&product_id);
    }
    if pattern.is_some() {
        for _ in 0..3 {
            count_query = count_query.bind(&pattern);
        }
    }
    let total = count_query
        .fetch_one(&pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(Page { items, total })
}

#[derive(Serialize)]
pub struct CsvImportError {
    pub row: usize,
    pub message: String,
}

#[derive(Serialize)]
pub struct CsvImportResult {
    pub imported: usize,
    pub errors: Vec<CsvImportError>,
}

fn normalize_header(raw: &str) -> String {
    raw.trim().to_lowercase().replace([' ', '-'], "_")
}

fn parse_import_f64(raw: &str, field: &str) -> Result<f64, String> {
    let t = raw.trim();
    if t.is_empty() {
        return Ok(0.0);
    }
    t.parse::<f64>()
        .map_err(|_| format!("Invalid {field} value \"{raw}\""))
}

fn parse_import_bool(raw: &str) -> Result<bool, String> {
    match raw.trim().to_lowercase().as_str() {
        "" | "0" | "no" | "n" | "false" => Ok(false),
        "1" | "yes" | "y" | "true" => Ok(true),
        _ => Err(format!(
            "Invalid quick_item value \"{raw}\" (use yes/no or 1/0)"
        )),
    }
}

fn validate_import_row(
    name: &str,
    barcode: &str,
    cost: f64,
    sell: f64,
    unit: &str,
    reorder: f64,
    stock: f64,
) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("Name is required".into());
    }
    if barcode.trim().is_empty() {
        return Err("Barcode is required".into());
    }
    if cost < 0.0 {
        return Err("Cost price cannot be negative".into());
    }
    if sell < 0.0 {
        return Err("Sell price cannot be negative".into());
    }
    if sell <= cost {
        return Err("Sell price must be more than cost price".into());
    }
    if unit.trim().is_empty() {
        return Err("Unit is required".into());
    }
    if reorder < 0.0 {
        return Err("Reorder level cannot be negative".into());
    }
    if stock < 0.0 {
        return Err("Stock cannot be negative".into());
    }
    Ok(())
}

#[tauri::command]
pub async fn import_products_csv<R: Runtime>(
    app: AppHandle<R>,
    source_path: String,
    user_id: Option<i64>,
) -> Result<CsvImportResult, String> {
    let pool = db::pool(&app).await?;
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(&source_path)
        .map_err(|e| format!("Could not read CSV file: {e}"))?;

    let headers = reader
        .headers()
        .map_err(|e| e.to_string())?
        .clone();
    let norm: Vec<String> = headers.iter().map(|h| normalize_header(h)).collect();

    let find_col = |aliases: &[&str]| -> Option<usize> {
        norm.iter().position(|h| aliases.contains(&h.as_str()))
    };
    let require_col = |aliases: &[&str], label: &str| -> Result<usize, String> {
        find_col(aliases).ok_or_else(|| format!("CSV is missing a '{label}' column"))
    };

    let c_name = require_col(&["name", "product", "productname", "product_name"], "name")?;
    let c_barcode = require_col(
        &["barcode", "code", "bar_code", "barcode_no"],
        "barcode",
    )?;
    let c_sku = find_col(&["sku", "product_code"]);
    let c_category = find_col(&["category", "categoryname", "category_name", "group"]);
    let c_cost = find_col(&["cost", "costprice", "cost_price"]);
    let c_sell = find_col(&["sell", "sellprice", "sell_price", "price"]);
    let c_unit = find_col(&["unit", "uom"]);
    let c_reorder = find_col(&["reorder", "reorderlevel", "reorder_level", "min_stock"]);
    let c_stock = find_col(&[
        "stock", "qty", "quantity", "openingstock", "opening_stock", "stockqty", "stock_qty",
    ]);
    let c_quick = find_col(&[
        "quick",
        "quickitem",
        "quick_item",
        "isquick",
        "is_quick",
    ]);

    let mut result = CsvImportResult {
        imported: 0,
        errors: Vec::new(),
    };
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    for (idx, record) in reader.records().enumerate() {
        let row_no = idx + 2;
        let record = match record {
            Ok(r) => r,
            Err(e) => {
                result
                    .errors
                    .push(CsvImportError { row: row_no, message: e.to_string() });
                continue;
            }
        };
        let cell = |i: Option<usize>| -> Option<String> {
            i.and_then(|ix| record.get(ix)).map(|s| s.trim().to_string())
        };

        let name = cell(Some(c_name)).unwrap_or_default();
        let barcode = cell(Some(c_barcode)).unwrap_or_default();

        let cost = match cell(c_cost).map(|v| parse_import_f64(&v, "cost price")).transpose() {
            Ok(v) => v.unwrap_or(0.0),
            Err(e) => {
                result.errors.push(CsvImportError { row: row_no, message: e });
                continue;
            }
        };
        let sell = match cell(c_sell).map(|v| parse_import_f64(&v, "sell price")).transpose() {
            Ok(v) => v.unwrap_or(0.0),
            Err(e) => {
                result.errors.push(CsvImportError { row: row_no, message: e });
                continue;
            }
        };
        let reorder = match cell(c_reorder).map(|v| parse_import_f64(&v, "reorder level")).transpose() {
            Ok(v) => v.unwrap_or(0.0),
            Err(e) => {
                result.errors.push(CsvImportError { row: row_no, message: e });
                continue;
            }
        };
        let stock = match cell(c_stock).map(|v| parse_import_f64(&v, "stock")).transpose() {
            Ok(v) => v.unwrap_or(0.0),
            Err(e) => {
                result.errors.push(CsvImportError { row: row_no, message: e });
                continue;
            }
        };
        let quick = match cell(c_quick).map(|v| parse_import_bool(&v)).transpose() {
            Ok(v) => v.unwrap_or(false),
            Err(e) => {
                result.errors.push(CsvImportError { row: row_no, message: e });
                continue;
            }
        };

        let unit = cell(c_unit)
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "store item".into());

        if let Err(msg) = validate_import_row(&name, &barcode, cost, sell, &unit, reorder, stock) {
            result.errors.push(CsvImportError { row: row_no, message: msg });
            continue;
        }

        let dup: Option<(i64,)> =
            sqlx::query_as("SELECT id FROM products WHERE barcode = ?")
                .bind(&barcode)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
        if dup.is_some() {
            result.errors.push(CsvImportError {
                row: row_no,
                message: format!("Product with barcode \"{barcode}\" already exists"),
            });
            continue;
        }

        let category_id = match cell(c_category).filter(|s| !s.is_empty()) {
            Some(cat_name) => {
                let existing: Option<(i64,)> = sqlx::query_as(
                    "SELECT id FROM categories WHERE lower(name) = lower(?)",
                )
                .bind(&cat_name)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
                match existing {
                    Some((id,)) => Some(id),
                    None => {
                        let r = sqlx::query("INSERT INTO categories (name) VALUES (?)")
                            .bind(&cat_name)
                            .execute(&mut *tx)
                            .await
                            .map_err(|e| e.to_string())?;
                        Some(r.last_insert_rowid())
                    }
                }
            }
            None => None,
        };

        let pid = sqlx::query(
            "INSERT INTO products
                (sku, barcode, name, category_id, cost_price, sell_price,
                 unit, stock_qty, reorder_level, is_quick)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(empty_to_none(&cell(c_sku)))
        .bind(&barcode)
        .bind(name.trim())
        .bind(category_id)
        .bind(cost)
        .bind(sell)
        .bind(unit.trim())
        .bind(stock)
        .bind(reorder)
        .bind(i64::from(quick))
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?
        .last_insert_rowid();

        if stock > 0.0 {
            sqlx::query(
                "INSERT INTO stock_movements (product_id, type, qty, notes)
                 VALUES (?, 'opening', ?, 'Imported from CSV')",
            )
            .bind(pid)
            .bind(stock)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
        }

        result.imported += 1;
    }

    tx.commit().await.map_err(|e| e.to_string())?;

    if result.imported > 0 {
        sqlx::query(
            "INSERT INTO audit_log (user_id, action, entity_type, entity_id, details)
             VALUES (?, 'product.import', 'product', NULL, ?)",
        )
        .bind(user_id)
        .bind(format!("Imported {} product(s) from CSV", result.imported))
        .execute(&pool)
        .await
        .map_err(|e| e.to_string())?;
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_import_bool_accepts_common_values() {
        for v in ["1", "yes", "y", "true", "YES", "True"] {
            assert!(parse_import_bool(v).unwrap(), "expected true for {v}");
        }
        for v in ["0", "no", "n", "false", "", "NO"] {
            assert!(!parse_import_bool(v).unwrap(), "expected false for {v}");
        }
        assert!(parse_import_bool("maybe").is_err());
    }
}
