# Store POS

Offline-first point-of-sale desktop app for small stores. Built with **Tauri 2**, **Vue 3 + TypeScript**, and a local **SQLite** database — no server required, your data never leaves the machine.

## Highlights

- Fast checkout with barcode/scan search, split payments, hold & resume
- Inventory, categories, suppliers, purchases, stock movements
- Customers with tab (credit) balances, ledger, payments, debt collection view
- Sales reports: revenue trends, top products, category mix, margins, exports
- Expenses tracking with categories
- Cashier sessions with opening/closing cash and variance
- Backups & restore with retention policy; full Excel accounting export
- Bilingual UI (English / العربية) with RTL support, dark & light themes
- Role-based permissions for admins and cashiers

## Requirements

- Node.js 20+
- Rust stable (MSVC toolchain on Windows)
- Platform prerequisites for Tauri: <https://tauri.app/start/prerequisites/>

## Development

```bash
npm install
npm run tauri dev     # launches the desktop app with hot reload
```

The SQLite database lives at:

| OS      | Path                                              |
| ------- | ------------------------------------------------- |
| Windows | `%APPDATA%\com.mohammad.storeapp\store.db`        |
| macOS   | `~/Library/Application Support/com.mohammad.storeapp` |
| Linux   | `~/.config/com.mohammad.storeapp`                 |

Default login on a fresh database is created by the seeder (see first-run screen).

## Build installers

```bash
npm run tauri build
```

Outputs (per current OS) land in `src-tauri/target/release/bundle/`:

- Windows: MSI (`msi/`) and NSIS setup (`nsis/`)
- macOS: DMG (`dmg/`)
- Linux: `.deb` (`deb/`) and AppImage (`appimage/`)

## Tests

```bash
cargo test            # Rust unit + integration tests (reports, customers, backups, smoke flow)
npx vue-tsc -b        # frontend type check
npx vite build        # production bundle
```

> Note: some corporate machines block Tauri's build script under test profiling (`os error 4551`). If `cargo test` fails that way, run it from an unrestricted shell.

## Project layout

```
src/                  Vue frontend
  app/presentation/   Pages (checkout, products, sales, reports, settings…)
  stores/             Pinia stores (cart, catalog, settings, auth, theme)
  lib/                db access, receipt printing, sound, seed helpers
  i18n/               en / ar locales
src-tauri/
  src/commands/       Tauri commands (sales, catalog, reports, backup, …)
  migrations/         SQL schema (001_initial.sql)
  tests/              Integration tests
plan.md               Feature plan & progress checklist
docs/USER_GUIDE.md    End-user documentation
```

## License

All rights reserved by the project owner.
