export interface AuthUser {
  id: number;
  username: string;
  fullName: string;
  roleName: string;
  permissions: string[];
}

export interface Role {
  id: number;
  name: string;
}

export interface UserRecord {
  id: number;
  username: string;
  fullName: string;
  roleId: number;
  roleName: string;
  isActive: boolean;
  permissions: string[];
}

export interface SettingsMap {
  [key: string]: string;
}

export interface CartItem {
  productId: number;
  name: string;
  price: number;
  qty: number;
  discount: number;
}

export interface Category {
  id: number;
  name: string;
  parentId?: number | null;
  productCount?: number;
}

export interface Product {
  id: number;
  sku: string | null;
  barcode: string | null;
  name: string;
  description: string | null;
  category_id: number | null;
  cost_price: number;
  sell_price: number;
  tax_profile_id: number | null;
  unit: string;
  stock_qty: number;
  reorder_level: number;
  image_path: string | null;
  is_active: number;
}

export interface TaxProfile {
  id: number;
  name: string;
  rate: number;
}

export interface Supplier {
  id: number;
  name: string;
  contact: string | null;
  phone: string | null;
  email: string | null;
  address: string | null;
  tax_id: string | null;
  created_at: string;
  invoice_count: number;
  total_purchased: number;
  total_due: number;
}

export interface SupplierInvoiceSummary {
  id: number;
  invoice_no: string;
  date: string;
  total: number;
  paid_amount: number;
  due_amount: number;
  status: string;
}

export interface SupplierDetail {
  supplier: Supplier;
  invoices: SupplierInvoiceSummary[];
}

export interface SupplierInvoice {
  id: number;
  invoice_no: string;
  supplier_id: number;
  supplier_name: string;
  date: string;
  total: number;
  paid_amount: number;
  due_amount: number;
  status: string;
  notes: string | null;
}

export interface InvoiceLine {
  productId: number | null;
  qty: number;
  costPrice: number;
}

export interface ExpenseCategory {
  id: number;
  name: string;
  expenseCount: number;
}

export interface OutgoingExpense {
  id: number;
  category_id: number | null;
  category_name: string | null;
  amount: number;
  date: string;
  description: string | null;
  reference_no: string | null;
  user_name: string | null;
}
