import type { SaleReceipt } from "../types";
import { i18n } from "../i18n";

/** Format a number as a plain currency string (no thousands grouping). */
function money(n: number): string {
  return n.toFixed(2);
}

/** Escape text for safe embedding in the receipt document. */
function esc(s: string | null | undefined): string {
  return (s ?? "")
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

function fmtDateTime(raw: string): string {
  const date = new Date(raw + (raw.includes("T") ? "" : "Z"));
  if (isNaN(date.getTime())) return raw;
  return date.toLocaleString(i18n.global.locale.value);
}

function methodLabel(method: string): string {
  switch (method) {
    case "cash":
      return t("receipt.methodCash");
    case "card":
      return t("receipt.methodCard");
    case "credit":
      return t("receipt.methodCredit");
    default:
      return esc(method).toUpperCase();
  }
}

const t = i18n.global.t;

/**
 * Builds a self-contained 80mm thermal receipt document from a sale. Uses a
 * fixed monospace layout so it prints cleanly on receipt printers.
 */
export function buildReceiptHtml(r: SaleReceipt): string {
  const statusLine =
    r.status === "voided"
      ? `<div class="center b">*** ${esc(t("receipt.voided"))} ***</div>`
      : r.status !== "completed"
        ? `<div class="center b">*** ${esc(r.status.toUpperCase())} ***</div>`
        : "";

  const itemRows = r.items
    .map((it) => {
      const name = esc(it.name);
      const line = `${it.qty} x ${money(it.price)}`;
      const discountLine =
        it.discount > 0
          ? `<div class="row sub"><span>${esc(t("receipt.discount"))}</span><span>-${money(it.discount * it.qty)}</span></div>`
          : "";
      return `
      <div class="item-name">${name}</div>
      <div class="row"><span>${esc(line)}</span><span>${money(it.subtotal)}</span></div>
      ${discountLine}`;
    })
    .join("");

  const discountRows = r.itemDiscount > 0
    ? `<div class="row sub"><span>${esc(t("receipt.itemDiscounts"))}</span><span>-${money(r.itemDiscount)}</span></div>`
    : "";
  const orderDiscountRows = r.orderDiscount > 0
    ? `<div class="row sub"><span>${esc(t("receipt.orderDiscount"))}</span><span>-${money(r.orderDiscount)}</span></div>`
    : "";
  const taxRows = r.tax > 0
    ? `<div class="row"><span>${esc(t("receipt.tax"))}</span><span>${money(r.tax)}</span></div>`
    : "";

  const paymentRows = r.payments
    .map(
      (p) =>
        `<div class="row"><span>${methodLabel(p.method)}</span><span>${money(p.amount)}</span></div>`
    )
    .join("");

  const customerRow = r.customerName
    ? `<div class="row"><span>${esc(t("common.customer"))}</span><span>${esc(r.customerName)}</span></div>`
    : "";
  const cashierRow = r.userName
    ? `<div class="row"><span>${esc(t("common.cashier"))}</span><span>${esc(r.userName)}</span></div>`
    : "";

  const storeHeader = r.storeName || t("receipt.receipt");
  const storeLine1 = r.storeAddress ? `<div class="center">${esc(r.storeAddress)}</div>` : "";
  const storeLine2 = r.storePhone ? `<div class="center">${esc(r.storePhone)}</div>` : "";
  const storeLine3 = r.storeTaxId
    ? `<div class="center">${esc(t("suppliers.taxId"))}: ${esc(r.storeTaxId)}</div>`
    : "";
  const footer = r.receiptFooter || t("receipt.footer");

  return `<!doctype html>
<html>
<head>
<meta charset="utf-8">
<title>${esc(t("receipt.receipt"))} ${esc(r.saleNo)}</title>
<style>
  @page { size: 80mm auto; margin: 2mm; }
  * { margin: 0; padding: 0; box-sizing: border-box; }
  html, body { background: #fff; }
  body {
    width: 72mm;
    margin: 0 auto;
    font-family: "Courier New", Courier, monospace;
    font-size: 11px;
    line-height: 1.35;
    color: #000;
  }
  .center { text-align: center; }
  .b { font-weight: bold; }
  .banner { font-size: 15px; margin-bottom: 2px; }
  .row { display: flex; justify-content: space-between; gap: 4px; }
  .row.sub { font-size: 10px; }
  .hr { border-top: 1px dashed #000; margin: 5px 0; }
  .item-name { white-space: pre-wrap; }
  .footer { margin-top: 6px; text-align: center; font-size: 10px; }
</style>
</head>
<body>
  <div class="center b banner">${esc(storeHeader)}</div>
  ${storeLine1}
  ${storeLine2}
  ${storeLine3}
  <div class="hr"></div>
  ${statusLine}
  <div class="row"><span>${esc(t("receipt.saleNo"))}</span><span>${esc(r.saleNo)}</span></div>
  <div class="row"><span>${esc(t("common.date"))}</span><span>${esc(fmtDateTime(r.createdAt))}</span></div>
  ${cashierRow}
  ${customerRow}
  <div class="hr"></div>
  ${itemRows}
  <div class="hr"></div>
  <div class="row"><span>${esc(t("receipt.subtotal"))}</span><span>${money(r.subtotal)}</span></div>
  ${discountRows}
  ${orderDiscountRows}
  ${taxRows}
  <div class="row b" style="font-size: 13px"><span>${esc(t("receipt.total"))}</span><span>${money(r.total)}</span></div>
  <div class="hr"></div>
  ${paymentRows}
  <div class="row"><span>${esc(t("receipt.paid"))}</span><span>${money(r.paidAmount)}</span></div>
  <div class="row"><span>${esc(t("receipt.change"))}</span><span>${money(r.changeGiven)}</span></div>
  <div class="hr"></div>
  <div class="footer">${esc(footer)}</div>
</body>
</html>`;
}

let printFrame: HTMLIFrameElement | null = null;

/**
 * Renders `html` into an off-screen iframe and opens the OS print dialog for
 * it. Resolves once the dialog has been dismissed.
 */
export function printReceipt(html: string): Promise<void> {
  return new Promise((resolve) => {
    if (!printFrame) {
      printFrame = document.createElement("iframe");
      printFrame.style.position = "fixed";
      printFrame.style.right = "0";
      printFrame.style.bottom = "0";
      printFrame.style.width = "0";
      printFrame.style.height = "0";
      printFrame.style.border = "0";
      printFrame.style.opacity = "0";
      printFrame.setAttribute("aria-hidden", "true");
      document.body.appendChild(printFrame);
    }
    const frame = printFrame;
    let printed = false;
    const go = () => {
      if (printed) return;
      printed = true;
      try {
        const win = frame.contentWindow;
        if (win) {
          win.focus();
          win.print();
        }
      } catch {
        // printing is unavailable in this environment
      }
      resolve();
    };
    frame.onload = go;
    frame.srcdoc = html;
    window.setTimeout(go, 400);
  });
}
