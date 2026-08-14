PRAGMA foreign_keys = ON;

-- ============================================================
-- 5.1 System & Settings
-- ============================================================
CREATE TABLE settings (
  key        TEXT PRIMARY KEY,
  value      TEXT NOT NULL
);

CREATE TABLE currencies (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  code       TEXT UNIQUE NOT NULL,
  name       TEXT NOT NULL,
  symbol     TEXT NOT NULL,
  rate       REAL NOT NULL DEFAULT 1,
  is_base    INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE tax_profiles (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  name       TEXT UNIQUE NOT NULL,
  rate       REAL NOT NULL DEFAULT 0,
  is_default INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE roles (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  name        TEXT UNIQUE NOT NULL,
  description TEXT
);

CREATE TABLE permissions (
  id   INTEGER PRIMARY KEY AUTOINCREMENT,
  code TEXT UNIQUE NOT NULL
);

CREATE TABLE role_permissions (
  role_id       INTEGER NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
  permission_id INTEGER NOT NULL REFERENCES permissions(id) ON DELETE CASCADE,
  PRIMARY KEY (role_id, permission_id)
);

CREATE TABLE users (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  username      TEXT UNIQUE NOT NULL,
  password_hash TEXT NOT NULL,
  full_name     TEXT NOT NULL,
  role_id       INTEGER NOT NULL REFERENCES roles(id),
  is_active     INTEGER NOT NULL DEFAULT 1,
  created_at    TEXT NOT NULL DEFAULT (datetime('now'))
);

-- ============================================================
-- 5.2 Product Catalog & Inventory
-- ============================================================
CREATE TABLE categories (
  id        INTEGER PRIMARY KEY AUTOINCREMENT,
  name      TEXT UNIQUE NOT NULL,
  parent_id INTEGER REFERENCES categories(id) ON DELETE SET NULL
);

CREATE TABLE products (
  id             INTEGER PRIMARY KEY AUTOINCREMENT,
  sku            TEXT,
  barcode        TEXT,
  name           TEXT NOT NULL,
  description    TEXT,
  category_id    INTEGER REFERENCES categories(id) ON DELETE SET NULL,
  cost_price     REAL NOT NULL DEFAULT 0,
  sell_price     REAL NOT NULL DEFAULT 0,
  tax_profile_id INTEGER REFERENCES tax_profiles(id) ON DELETE SET NULL,
  unit           TEXT NOT NULL DEFAULT 'pc',
  stock_qty      REAL NOT NULL DEFAULT 0,
  reorder_level  REAL NOT NULL DEFAULT 0,
  image_path     TEXT,
  is_active      INTEGER NOT NULL DEFAULT 1,
  created_at     TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at     TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE UNIQUE INDEX idx_products_sku     ON products(sku)     WHERE sku IS NOT NULL;
CREATE INDEX        idx_products_barcode ON products(barcode) WHERE barcode IS NOT NULL;

CREATE TABLE stock_movements (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  product_id INTEGER NOT NULL REFERENCES products(id) ON DELETE CASCADE,
  type       TEXT NOT NULL,
  qty        REAL NOT NULL,
  ref_id     INTEGER,
  notes      TEXT,
  user_id    INTEGER REFERENCES users(id),
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_stock_movements_product ON stock_movements(product_id);

-- ============================================================
-- 5.3 Suppliers & Purchasing
-- ============================================================
CREATE TABLE suppliers (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  name       TEXT NOT NULL,
  contact    TEXT,
  phone      TEXT,
  email      TEXT,
  address    TEXT,
  tax_id     TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE supplier_invoices (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  invoice_no  TEXT NOT NULL,
  supplier_id INTEGER NOT NULL REFERENCES suppliers(id),
  date        TEXT NOT NULL,
  total       REAL NOT NULL DEFAULT 0,
  paid_amount REAL NOT NULL DEFAULT 0,
  due_amount  REAL NOT NULL DEFAULT 0,
  status      TEXT NOT NULL DEFAULT 'unpaid',
  notes       TEXT,
  user_id     INTEGER REFERENCES users(id),
  created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE supplier_invoice_items (
  id           INTEGER PRIMARY KEY AUTOINCREMENT,
  invoice_id   INTEGER NOT NULL REFERENCES supplier_invoices(id) ON DELETE CASCADE,
  product_id   INTEGER NOT NULL REFERENCES products(id),
  qty          REAL NOT NULL,
  cost_price   REAL NOT NULL,
  subtotal     REAL NOT NULL
);

CREATE TABLE supplier_payments (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  invoice_id INTEGER NOT NULL REFERENCES supplier_invoices(id) ON DELETE CASCADE,
  amount     REAL NOT NULL,
  method     TEXT NOT NULL DEFAULT 'cash',
  date       TEXT NOT NULL DEFAULT (datetime('now')),
  notes      TEXT,
  user_id    INTEGER REFERENCES users(id)
);

CREATE TABLE expense_categories (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  name       TEXT UNIQUE NOT NULL
);

CREATE TABLE expense_out (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  category_id   INTEGER REFERENCES expense_categories(id) ON DELETE SET NULL,
  amount        REAL NOT NULL,
  date          TEXT NOT NULL DEFAULT (datetime('now')),
  description   TEXT,
  reference_no  TEXT,
  user_id       INTEGER REFERENCES users(id),
  created_at    TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_expense_out_date ON expense_out(date);

-- ============================================================
-- 5.4 Sales / Checkout
-- ============================================================
CREATE TABLE sale_sessions (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  user_id       INTEGER NOT NULL REFERENCES users(id),
  opened_at     TEXT NOT NULL DEFAULT (datetime('now')),
  closed_at     TEXT,
  opening_cash  REAL NOT NULL DEFAULT 0,
  closing_cash  REAL,
  expected_cash REAL,
  status        TEXT NOT NULL DEFAULT 'open'
);

CREATE TABLE sales (
  id           INTEGER PRIMARY KEY AUTOINCREMENT,
  sale_no      TEXT UNIQUE NOT NULL,
  session_id   INTEGER REFERENCES sale_sessions(id),
  customer_id  INTEGER REFERENCES customers(id),
  user_id      INTEGER NOT NULL REFERENCES users(id),
  currency_id  INTEGER REFERENCES currencies(id),
  subtotal     REAL NOT NULL DEFAULT 0,
  discount     REAL NOT NULL DEFAULT 0,
  tax          REAL NOT NULL DEFAULT 0,
  total        REAL NOT NULL DEFAULT 0,
  paid_amount  REAL NOT NULL DEFAULT 0,
  change_given REAL NOT NULL DEFAULT 0,
  status       TEXT NOT NULL DEFAULT 'completed',
  void_reason  TEXT,
  voided_by    INTEGER REFERENCES users(id),
  created_at   TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_sales_created_at ON sales(created_at);

CREATE TABLE sale_items (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  sale_id    INTEGER NOT NULL REFERENCES sales(id) ON DELETE CASCADE,
  product_id INTEGER NOT NULL REFERENCES products(id),
  qty        REAL NOT NULL,
  price      REAL NOT NULL,
  cost_price REAL NOT NULL,
  discount   REAL NOT NULL DEFAULT 0,
  tax        REAL NOT NULL DEFAULT 0,
  subtotal   REAL NOT NULL
);

CREATE TABLE payments (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  sale_id    INTEGER NOT NULL REFERENCES sales(id) ON DELETE CASCADE,
  method     TEXT NOT NULL,
  amount     REAL NOT NULL,
  reference  TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- ============================================================
-- 5.5 Customers & Debt Tracking
-- ============================================================
CREATE TABLE customers (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  name       TEXT NOT NULL,
  phone      TEXT,
  email      TEXT,
  address    TEXT,
  balance    REAL NOT NULL DEFAULT 0,
  notes      TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE customer_ledger (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  customer_id   INTEGER NOT NULL REFERENCES customers(id) ON DELETE CASCADE,
  sale_id       INTEGER REFERENCES sales(id),
  type          TEXT NOT NULL,
  amount        REAL NOT NULL,
  balance_after REAL NOT NULL,
  notes         TEXT,
  user_id       INTEGER REFERENCES users(id),
  created_at    TEXT NOT NULL DEFAULT (datetime('now'))
);

-- ============================================================
-- 5.6 Audit & Reports
-- ============================================================
CREATE TABLE audit_log (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  user_id     INTEGER REFERENCES users(id),
  action      TEXT NOT NULL,
  entity_type TEXT,
  entity_id   INTEGER,
  details     TEXT,
  created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_audit_log_user ON audit_log(user_id);
CREATE INDEX idx_audit_log_action ON audit_log(action);

CREATE TABLE saved_reports (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  name        TEXT NOT NULL,
  type        TEXT NOT NULL,
  params      TEXT,
  start_date  TEXT,
  end_date    TEXT,
  created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE backups (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  file_path   TEXT NOT NULL,
  size_bytes  INTEGER,
  status      TEXT NOT NULL DEFAULT 'completed',
  created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

-- ============================================================
-- 5.7 Recommended Indexes (summary)
-- ============================================================
CREATE INDEX idx_sale_items_sale             ON sale_items(sale_id);
CREATE INDEX idx_payments_sale               ON payments(sale_id);
CREATE INDEX idx_sales_customer              ON sales(customer_id);
CREATE INDEX idx_sales_user                  ON sales(user_id);
CREATE INDEX idx_sales_session               ON sales(session_id);
CREATE INDEX idx_ledger_customer             ON customer_ledger(customer_id);
CREATE INDEX idx_invoices_supplier           ON supplier_invoices(supplier_id);
CREATE INDEX idx_products_category           ON products(category_id);
CREATE INDEX idx_expense_out_category        ON expense_out(category_id);
CREATE INDEX idx_supplier_payments_invoice   ON supplier_payments(invoice_id);
