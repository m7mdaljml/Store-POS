export const PERMISSION_LABELS: Record<string, string> = {
  "sales.checkout": "Sell items (checkout)",
  "sales.void": "Void / refund sales",
  "sales.discount": "Apply item & order discounts",
  "reports.view": "View sales reports",
  "inventory.view": "View inventory & stock",
  "expenses.manage": "Record & manage expenses",
  "export.excel": "Export reports to Excel",
  "users.manage": "Manage cashiers & roles",
  "settings.manage": "Change store settings",
};

export function permissionLabel(code: string): string {
  return PERMISSION_LABELS[code] ?? code;
}
