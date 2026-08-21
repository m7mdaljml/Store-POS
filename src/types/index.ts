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
  /** Cost price snapshot at the time the item was added. */
  costPrice: number;
}

/** A non-cash payment line (card / customer credit). */
export interface PaymentLine {
  method: "card" | "credit";
  amount: number;
  reference?: string | null;
  customerId?: number | null;
}

export interface CustomerLite {
  id: number;
  name: string;
  phone: string | null;
  balance: number;
}

/** Result of a committed sale (from the `create_sale` command). */
export interface SaleResult {
  saleId: number;
  saleNo: string;
  subtotal: number;
  discount: number;
  tax: number;
  total: number;
  paidAmount: number;
  changeGiven: number;
}

/** A row from the sales history (`list_sales` command). */
export interface SaleRecord {
  id: number;
  saleNo: string;
  createdAt: string;
  userName: string | null;
  customerName: string | null;
  itemCount: number;
  total: number;
  paidAmount: number;
  status: "completed" | "voided" | "held" | "cancelled";
  voidReason: string | null;
}

/** A cash register session (shift). */
export interface SaleSession {
  id: number;
  userName: string | null;
  openedAt: string;
  closedAt: string | null;
  openingCash: number;
  closingCash: number | null;
  expectedCash: number | null;
  variance: number | null;
  status: "open" | "closed";
  salesCount: number;
  salesTotal: number;
  cashPaid: number;
  changeGiven: number;
}

/** Result of holding a sale (`hold_sale` command). */
export interface HoldSaleResult {
  saleId: number;
  saleNo: string;
  total: number;
  itemCount: number;
}

/** A single line of a resumed held sale. */
export interface ResumeSaleItem {
  productId: number;
  name: string;
  qty: number;
  price: number;
  costPrice: number;
  discount: number;
}

/** Held cart returned by `resume_sale`. */
export interface ResumeSaleRecord {
  saleId: number;
  saleNo: string;
  customerId: number | null;
  subtotal: number;
  discount: number;
  tax: number;
  total: number;
  items: ResumeSaleItem[];
}

/** A sale line item as returned on a receipt. */
export interface SaleReceiptItem {
  name: string;
  qty: number;
  price: number;
  discount: number;
  subtotal: number;
}

/** A payment method/amount line as returned on a receipt. */
export interface SaleReceiptPayment {
  method: string;
  amount: number;
  reference: string | null;
}

/** Everything needed to render a sale's receipt (`get_sale_receipt`). */
export interface SaleReceipt {
  storeName: string;
  storeAddress: string;
  storePhone: string;
  storeTaxId: string;
  receiptFooter: string;
  saleId: number;
  saleNo: string;
  createdAt: string;
  status: string;
  customerName: string | null;
  userName: string | null;
  subtotal: number;
  itemDiscount: number;
  orderDiscount: number;
  tax: number;
  total: number;
  paidAmount: number;
  changeGiven: number;
  items: SaleReceiptItem[];
  payments: SaleReceiptPayment[];
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

export interface Customer {
  id: number;
  name: string;
  phone: string | null;
  email: string | null;
  address: string | null;
  balance: number;
  notes: string | null;
  created_at: string;
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

export interface SupplierPayment {
  id: number;
  invoice_id: number;
  invoice_no: string;
  amount: number;
  method: string;
  date: string;
  notes: string | null;
  user_name: string | null;
}

export interface PaymentResult {
  paid_amount: number;
  due_amount: number;
  status: string;
}

export interface ExpenseRecord {
  kind: "in" | "out";
  id: number;
  ref_no: string | null;
  supplier_id: number | null;
  supplier_name: string | null;
  date: string;
  amount: number;
  paid_amount: number;
  due_amount: number;
  status: string;
  notes: string | null;
}

export interface ExpenseSummary {
  total_in: number;
  total_out: number;
  outstanding_due: number;
  incoming_count: number;
  outgoing_count: number;
}
