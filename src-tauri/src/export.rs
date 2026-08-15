use rust_xlsxwriter::{Format, Workbook};
use std::path::Path;

/// Writes a flat table (headers + string rows) to an `.xlsx` workbook on the
/// first sheet, with a bold header row and auto-sized columns. Shared by all
/// export features (expenses, sales, inventory, …).
pub fn write_xlsx(
    path: &Path,
    sheet_name: &str,
    headers: &[&str],
    rows: &[Vec<String>],
) -> Result<(), String> {
    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet();
    sheet.set_name(sheet_name).map_err(|e| e.to_string())?;

    let header_format = Format::new().set_bold().set_background_color("D9E1F2");

    for (col, header) in headers.iter().enumerate() {
        sheet
            .write_string_with_format(0, col as u16, *header, &header_format)
            .map_err(|e| e.to_string())?;
    }
    for (row_index, row) in rows.iter().enumerate() {
        for (col, value) in row.iter().enumerate() {
            sheet
                .write_string(row_index as u32 + 1, col as u16, value)
                .map_err(|e| e.to_string())?;
        }
    }

    for col in 0..headers.len() {
        let width = max_col_width(col, headers, rows).max(8.0).min(45.0) + 2.0;
        sheet
            .set_column_width(col as u16, width)
            .map_err(|e| e.to_string())?;
    }

    workbook.save(path).map_err(|e| e.to_string())?;
    Ok(())
}

fn max_col_width(col: usize, headers: &[&str], rows: &[Vec<String>]) -> f64 {
    let mut width = headers
        .get(col)
        .map(|h| h.len() as f64)
        .unwrap_or(0.0);
    for row in rows {
        if let Some(value) = row.get(col) {
            width = width.max(value.chars().count() as f64);
        }
    }
    width
}
