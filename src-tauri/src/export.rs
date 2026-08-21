use rust_xlsxwriter::{Format, Workbook};
use std::path::Path;

/// One worksheet worth of data: sheet name, header row and string cells.
#[derive(Debug, Clone)]
pub struct SheetData {
    pub name: String,
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

impl SheetData {
    pub fn new(name: &str, headers: &[&str]) -> Self {
        Self {
            name: name.to_string(),
            headers: headers.iter().map(|h| h.to_string()).collect(),
            rows: Vec::new(),
        }
    }
}

/// Writes a flat table (headers + string rows) to an `.xlsx` workbook on the
/// first sheet, with a bold header row and auto-sized columns. Shared by all
/// export features (expenses, sales, inventory, …).
pub fn write_xlsx(
    path: &Path,
    sheet_name: &str,
    headers: &[&str],
    rows: &[Vec<String>],
) -> Result<(), String> {
    let mut sheet = SheetData::new(sheet_name, headers);
    sheet.rows = rows.to_vec();
    write_xlsx_multi(path, &[sheet])
}

/// Writes several sheets into a single `.xlsx` workbook (F8.4 accounting handoff).
pub fn write_xlsx_multi(path: &Path, sheets: &[SheetData]) -> Result<(), String> {
    let mut workbook = Workbook::new();
    let header_format = Format::new().set_bold().set_background_color("D9E1F2");

    for data in sheets {
        let sheet = workbook.add_worksheet();
        sheet
            .set_name(&data.name)
            .map_err(|e| e.to_string())?;

        for (col, header) in data.headers.iter().enumerate() {
            sheet
                .write_string_with_format(0, col as u16, header.as_str(), &header_format)
                .map_err(|e| e.to_string())?;
        }
        for (row_index, row) in data.rows.iter().enumerate() {
            for (col, value) in row.iter().enumerate() {
                sheet
                    .write_string(row_index as u32 + 1, col as u16, value)
                    .map_err(|e| e.to_string())?;
            }
        }

        for col in 0..data.headers.len() {
            let width = max_col_width(col, &data.headers, &data.rows).max(8.0).min(45.0) + 2.0;
            sheet
                .set_column_width(col as u16, width)
                .map_err(|e| e.to_string())?;
        }
    }

    workbook.save(path).map_err(|e| e.to_string())?;
    Ok(())
}

fn max_col_width(col: usize, headers: &[String], rows: &[Vec<String>]) -> f64 {
    let mut width = headers
        .get(col)
        .map(|h| h.chars().count() as f64)
        .unwrap_or(0.0);
    for row in rows {
        if let Some(value) = row.get(col) {
            width = width.max(value.chars().count() as f64);
        }
    }
    width
}
