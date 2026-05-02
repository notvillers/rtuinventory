# AGENTS guide for rtuinventory

## Quick reference

**Build & test:**
- `cargo build` – compile and catch errors
- `cargo run` – run the TUI
- `cargo test` – no committed tests; runs 0 tests
- `cargo fmt` – format code
- `cargo fmt -- --check` – formatting check only

## Architecture at a glance

**Single-binary TUI with two screens:**
- `src/main.rs` owns terminal lifecycle, event loop, focus routing, and rendering of both Inventory and Settings screens.
- `App` struct holds: `Vec<Item>`, table/list state, input fields, status, edit/delete mode, current `Connection`, `Settings` object, current `Screen`, and settings-specific state.
- Startup: `App::new()` → `Settings::load()` → `connect_db(path)` → schema setup → `get_items()`.
- Two screens (Screen enum): `Inventory` (default) and `Settings`.
- Keyboard globals: `Ctrl+S` toggles screens, `Esc` closes settings, `q` quits.

**Persistence layer (`src/db/`):**
- `sql.rs` – SQL constants (CREATE, SELECT, INSERT, DELETE).
- `db.rs` – `Item` and `InsertItem` types, `connect_db(path)`, `get_items()`, `try_create_item()`, `try_delete_item()`.
- `mod.rs` – module wiring only.

**Settings (`src/settings.rs`):**
- `Settings` struct with `DatabaseSettings` (path, recent list).
- `Settings::load()` reads `app-settings.toml` or returns defaults.
- `set_database_path()` updates current path, maintains recent list (max 10), saves to file.
- Default database: `inventory.sqlite3`.
- TOML format: simple, human-editable.

**No service layer:** Add/edit/delete operations write to SQLite and sync `app.items` in the same handler.

**Inventory layout:** Single table (top, flexible) + bottom control area (name/qty, location, barcode/serial, buttons, help).

**Settings layout:** Recent databases list (top) + new database path input (bottom).

## When making changes

**Terminal lifecycle (critical):**
- `enable_raw_mode` ↔ `disable_raw_mode`
- `EnterAlternateScreen` ↔ `LeaveAlternateScreen`
- `show_cursor` on exit
- If unbalanced, terminal left in bad state.

**Database changes:**
- Always use `connect_db(path)` with the path parameter (not hardcoded).
- Always use `get_items()`, `try_create_item()`, `try_delete_item()`.
- Schema compatibility handled inside `connect_db()` via `PRAGMA table_info(...)`.
- Never duplicate schema setup.

**Screen switching:**
- Use `Screen` enum to route rendering and input handling.
- `draw_ui()` dispatches to `draw_inventory_ui()` or `draw_settings_ui()`.
- `handle_inventory_input()` and `handle_settings_input()` route keyboard.

**Database switching:**
- Call `switch_database(path)` in settings handlers.
- This: closes old connection, updates settings, reconnects, reloads items, clears UI state.
- Persists path to `app-settings.toml`.

**UI state sync:**
- `app.items` is the in-memory source of truth for rendering.
- If you mutate SQLite, update the corresponding `app.items` entry in the same handler.
- When switching databases, wipe all input fields, editing state, and delete confirmations.

**Input normalization:**
- Empty `barcode`, `serial`, `location` → `None`
- Quantity parsed as `u32`; blank quantity = `0`

**Table selection:**
- Use `TableState::default()` and `TableState::select(Some(index))`.
- Keep selection valid after add/delete (adjust index if needed).

**Focus and keyboard:**
- Inventory screen: `Focus` enum routes input; `active_input_mut()` returns mutable ref to focused field.
- Settings screen: `SettingsFocus` enum routes input to list or text field.
- When adding a new control, update:
  1. Focus cycling or screen-specific routing
  2. Keyboard handling in `handle_inventory_input()` or `handle_settings_input()`
  3. Visual styling in `draw_inventory_ui()` or `draw_settings_ui()`

**Edit and delete:**
- Edit mode: `editing_row` field transforms button and pre-fills inputs.
- Delete mode: Two-step via `pending_delete` (request → confirm y/n).

**Settings file:**
- Loaded once at startup via `Settings::load()`.
- Saved on every `set_database_path()` call.
- If missing or corrupted, defaults are used.
- Located at `app-settings.toml` in the working directory.

## Notes
- No CI workflows or pre-commit hooks.
- All changes verified through `cargo build` and `cargo run`.
- No external code generation or migration tools.
- Settings are user-persisted across sessions.

## Git commit policy

- **Only commit and push if you edit `.md` files (documentation).**
- If your changes include any code files (`.rs`, `.toml`, `Cargo.lock`, etc.), **do not commit or push**—leave them for the user to review and commit manually.
- This keeps the repository clean and gives the user full control over code changes, while documentation updates are safe to auto-commit.