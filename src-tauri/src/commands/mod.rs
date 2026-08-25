use serde::Serialize;

pub mod auth;
pub mod backup;
pub mod catalog;
pub mod customers;
pub mod expenses;
pub mod purchasing;
pub mod reports;
pub mod sales;
pub mod sessions;
pub mod suppliers;

/// Paginated list result: `items` is the requested page window and `total`
/// the exact filtered row count, so the UI can render true page numbers.
#[derive(Debug, Serialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total: i64,
}
