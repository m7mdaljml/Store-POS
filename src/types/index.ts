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
