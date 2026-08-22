# Store POS — User Guide

A practical walkthrough of every area of the app. Keyboard shortcuts and tips are noted where they apply.

## Getting started

1. Launch **Store POS**.
2. Sign in with your cashier account. Admins can manage users under **Users**.
3. Pick your language (EN/العربية) or theme from the top bar; both are remembered. You can also change them later in **Settings → Preferences**.

## Checkout (point of sale)

- Search by name, or scan a barcode — suggestions appear as you type.
- Click products to add them to the ticket. Adjust quantities/discounts inline.
- Attach a customer for tab (credit) sales; use **Charge to tab** when there is a shortfall.
- Pay with cash/card/split lines; change is computed automatically.
- **Hold** a sale to serve another customer, resume it anytime from the held list.
- Tick **Print receipt after sale** to print automatically on completion.

### Keyboard shortcuts

| Key | Action                                          |
| --- | ----------------------------------------------- |
| F2  | Focus product search (start a new sale)         |
| F5  | Jump to the cash-received field                 |
| F4  | Complete Sale                                   |
| Esc | Close popovers / clear error & success messages |
| F12 | Hold the current sale                           |

## Products & inventory

- **Products**: create/edit items ( barcode, cost/sell price, category, tax profile, unit, reorder level). Search and filter by status/category.
- **Stock**: receive stock via purchases, adjust with reasons; every change is recorded in stock movements (viewable per product).
- Low-stock items are highlighted on the Reports → Inventory page.

## Customers & debt

- Create customers with contact details.
- Credit sales increase their balance; payments reduce it.
- Open a customer to see purchase history + full ledger; record payments from the same dialog.
- The debt-collection panel lists everyone who owes money, sorted by amount.

## Suppliers & purchases

- Manage suppliers; log supplier invoices with items to receive stock and track payables.

## Sales & sessions

- **Sales**: browse history, filter by date/cashier/customer/status, void with reason (permission-gated).
- **Sessions**: open a session with opening cash at shift start; close with counted cash — variance is calculated.

## Expenses

- Record business expenses with categories; export to Excel for bookkeeping.

## Reports

- **Overview**: revenue, orders, average ticket, gross profit, expenses, net position — today / this week / this month / custom range.
- **Sales**: filterable detail table with totals.
- **Products**: best sellers by revenue/profit.
- **Inventory**: stock value, low-stock alerts, movements viewer.
- **Margins**: profit per category/product.
- Export any list to Excel using the green export buttons.

## Backups & data safety

- **Settings → Backups**
  - _Run check now_: verifies database health.
  - _Back up now_: snapshots the database into your chosen folder (default: app data folder). Each backup is logged below.
  - _Automatic backups_: enable daily/weekly auto-backups; old backups beyond "keep last N" are pruned automatically.
  - _Restore_: pick any backup row (or file) to replace the current database. A safety copy of the current data is taken first — you can always go back.
- **Export all data (Excel)**: one workbook with a sheet per table for accounting handoff.

## Settings

- **Store profile**: name, phone, address, tax ID and logo — used across the UI and receipts.
- **Tax profiles**: define VAT rates, mark one default; assign per product.
- **Currencies**: manage currencies and rates relative to the base currency; checkout shows a converted total for reference (sales are always recorded in base).
- **Receipt**: header/footer text, logo position, thermal 80mm vs A4 paper.
- **Preferences**: dark/light theme, language, UI sounds.

## Troubleshooting

- **"Database is locked"**: another operation holds the DB briefly; wait a second and retry. Persistent issues → restart the app.
- **Printing fails**: ensure a default printer is configured in the OS; receipts render in a print preview window.
- **Backup restore failed**: confirm the file is one produced by Store POS (`store-backup-*.db` / `pre-restore-*.db`) or an unmodified SQLite file.
