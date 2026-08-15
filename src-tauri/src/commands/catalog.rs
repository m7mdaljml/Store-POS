use serde::{Deserialize, Serialize};
use sqlx::Row;
use tauri::{AppHandle, Manager, Runtime};

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
    let result = sqlx::query("INSERT INTO categories (name, parent_id) VALUES (?, ?)")
        .bind(&name)
        .bind(parent_id)
        .execute(&pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(result.last_insert_rowid())
}

#[tauri::command]
pub async fn update_category<R: Runtime>(
    app: AppHandle<R>,
    category_id: i64,
    name: String,
    parent_id: Option<i64>,
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
    let result = sqlx::query("UPDATE categories SET name = ?, parent_id = ? WHERE id = ?")
        .bind(&name)
        .bind(parent_id)
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
pub async fn delete_category<R: Runtime>(
    app: AppHandle<R>,
    category_id: i64,
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
    let result = sqlx::query("DELETE FROM categories WHERE id = ?")
        .bind(category_id)
        .execute(&pool)
        .await
        .map_err(|e| e.to_string())?;
    if result.rows_affected() == 0 {
        return Err("Category not found".into());
    }
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
) -> Result<i64, String> {
    let pool = db::pool(&app).await?;
    validate_product(&input)?;
    ensure_sku_unique(&pool, &input.sku, None).await?;

    let result = sqlx::query(
        "INSERT INTO products
            (sku, barcode, name, description, category_id, cost_price, sell_price,
             tax_profile_id, unit, reorder_level, image_path, is_active)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
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
    .execute(&pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(result.last_insert_rowid())
}

#[tauri::command]
pub async fn update_product<R: Runtime>(
    app: AppHandle<R>,
    product_id: i64,
    input: ProductInput,
) -> Result<(), String> {
    let pool = db::pool(&app).await?;
    validate_product(&input)?;
    ensure_sku_unique(&pool, &input.sku, Some(product_id)).await?;

    let result = sqlx::query(
        "UPDATE products SET
            sku = ?, barcode = ?, name = ?, description = ?, category_id = ?,
            cost_price = ?, sell_price = ?, tax_profile_id = ?, unit = ?,
            reorder_level = ?, image_path = ?, is_active = ?
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
    .bind(product_id)
    .execute(&pool)
    .await
    .map_err(|e| e.to_string())?;
    if result.rows_affected() == 0 {
        return Err("Product not found".into());
    }
    Ok(())
}

#[tauri::command]
pub async fn delete_product<R: Runtime>(app: AppHandle<R>, product_id: i64) -> Result<(), String> {
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
    let result = sqlx::query("DELETE FROM products WHERE id = ?")
        .bind(product_id)
        .execute(&pool)
        .await
        .map_err(|e| e.to_string())?;
    if result.rows_affected() == 0 {
        return Err("Product not found".into());
    }
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
        "INSERT INTO stock_movements (product_id, type, qty, notes) VALUES (?, 'adjustment', ?, ?)",
    )
    .bind(product_id)
    .bind(qty)
    .bind(notes)
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query("UPDATE products SET stock_qty = ? WHERE id = ?")
        .bind(new_qty)
        .bind(product_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(())
}
