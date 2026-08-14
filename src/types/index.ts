export interface AuthUser {
  id: number;
  username: string;
  fullName: string;
  roleName: string;
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
}

export interface Product {
  id: number;
  sku: string | null;
  barcode: string | null;
  name: string;
  cost_price: number;
  sell_price: number;
  stock_qty: number;
  category_id: number | null;
}
