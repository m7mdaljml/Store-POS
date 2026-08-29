use rust_xlsxwriter::{Format, Image, Workbook};
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

/// A single export row that optionally carries an embedded thumbnail image.
/// The image is placed in its own leading column (empty header cell), separate
/// from the string `cells`. Used by product export so images survive a
/// round-trip import (F0.1/Catalog full-detail Excel).
#[derive(Debug, Clone)]
pub struct ImageRow {
    pub cells: Vec<String>,
    pub image: Option<Vec<u8>>,
    pub image_name: String,
}

/// Writes a worksheet whose first column holds an optional thumbnail image per
/// row (`row + 1`), followed by the string cells starting at column 1. Rows
/// without an image are still written, with a fixed height for visual
/// alignment. Column 0 is reserved for the image and skipped by the auto-width
/// pass (its width is fixed).
pub fn write_image_table(
    path: &Path,
    sheet_name: &str,
    headers: &[String],
    rows: &[ImageRow],
) -> Result<(), String> {
    let mut workbook = Workbook::new();
    let header_format = Format::new().set_bold().set_background_color("D9E1F2");
    let sheet = workbook.add_worksheet();
    sheet.set_name(sheet_name).map_err(|e| e.to_string())?;

    for (col, header) in headers.iter().enumerate() {
        sheet
            .write_string_with_format(0, col as u16, header.as_str(), &header_format)
            .map_err(|e| e.to_string())?;
    }

    // Fixed, reasonably-sized thumbnail column.
    sheet.set_column_width(0, 16.0).map_err(|e| e.to_string())?;

    for (i, row) in rows.iter().enumerate() {
        let r = i as u32 + 1;
        sheet.set_row_height(r, 64.0).map_err(|e| e.to_string())?;
        for (col, value) in row.cells.iter().enumerate() {
            sheet
                .write_string(r, col as u16 + 1, value)
                .map_err(|e| e.to_string())?;
        }
        if let Some(bytes) = &row.image {
            let mut img = Image::new_from_buffer(bytes).map_err(|e| e.to_string())?;
            img = img
                .set_scale_width(0.9)
                .set_scale_height(0.9)
                .set_alt_text(&row.image_name);
            sheet.insert_image(r, 0, &img).map_err(|e| e.to_string())?;
        }
    }

    // Auto-size the data columns (skip the reserved image column).
    for col in 0..headers.len() {
        let width = data_col_width(col, headers, rows).max(8.0).min(45.0) + 2.0;
        sheet.set_column_width(col as u16 + 1, width).map_err(|e| e.to_string())?;
    }

    workbook.save(path).map_err(|e| e.to_string())?;
    Ok(())
}

fn data_col_width(col: usize, headers: &[String], rows: &[ImageRow]) -> f64 {
    let mut width = headers
        .get(col)
        .map(|h| h.chars().count() as f64)
        .unwrap_or(0.0);
    for row in rows {
        if let Some(value) = row.cells.get(col) {
            width = width.max(value.chars().count() as f64);
        }
    }
    width
}

#[cfg(test)]
mod tests {
    use super::*;
    use calamine::{Data, Reader, Xlsx};
    use std::io::BufReader;

    fn cell_text(d: &Data) -> String {
        match d {
            Data::String(s) => s.clone(),
            Data::Int(i) => i.to_string(),
            Data::Float(f) => {
                if f.fract().abs() < f64::EPSILON {
                    format!("{}", *f as i64)
                } else {
                    f.to_string()
                }
            }
            Data::Bool(b) => {
                if *b {
                    "TRUE".into()
                } else {
                    "FALSE".into()
                }
            }
            Data::DateTime(edt) => edt.to_string(),
            Data::DateTimeIso(s) | Data::DurationIso(s) => s.clone(),
            Data::Error(_) | Data::Empty => String::new(),
        }
    }

    // A minimal valid 1x1 transparent PNG so the writer accepts it and calamine
    // can extract it back from the workbook's media.
    const TINY_PNG: &[u8] = &[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F,
        0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00,
        0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
        0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];

    #[test]
    fn image_table_round_trips_through_calamine() {
        let dir = std::env::temp_dir().join(format!("xlsx_rt_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("products.xlsx");

        let headers: Vec<String> = ["Image", "Name", "Barcode"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let rows = vec![
            ImageRow {
                cells: vec!["Apple".into(), "1001".into()],
                image: Some(TINY_PNG.to_vec()),
                image_name: "apple".into(),
            },
            ImageRow {
                // No image -> row must still write and read back without one.
                cells: vec!["Banana".into(), "1002".into()],
                image: None,
                image_name: "banana".into(),
            },
            ImageRow {
                cells: vec!["Cherry".into(), "1003".into()],
                image: Some(TINY_PNG.to_vec()),
                image_name: "cherry".into(),
            },
        ];

        write_image_table(&path, "Products", &headers, &rows).unwrap();

        let mut wb: Xlsx<BufReader<std::fs::File>> = calamine::open_workbook(&path).unwrap();
        let name = wb.sheet_names()[0].clone();
        let range = wb.worksheet_range(&name).unwrap();

        // Header row (image column is reserved at column 0).
        let mut it = range.rows();
        let header: Vec<String> = it.next().unwrap().iter().map(cell_text).collect();
        assert_eq!(header[1..], ["Name", "Barcode"]);

        // Data rows read back in order.
        let names: Vec<String> = it.map(|r| cell_text(&r[1])).collect();
        assert_eq!(names, ["Apple", "Banana", "Cherry"]);

        // Images are anchored to the correct 0-based row (header at row 0).
        let pics = wb.pictures_with_metadata();
        let anchored: Vec<u32> = pics.iter().map(|p| p.row).collect();
        assert!(anchored.contains(&1), "row 1 (Apple) has an image");
        assert!(anchored.contains(&3), "row 3 (Cherry) has an image");
        assert!(!anchored.contains(&2), "row 2 (Banana) has no image");
        assert!(pics.iter().all(|p| p.col == 0), "images anchored in column 0");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
