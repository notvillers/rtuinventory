# AGENTS guide for rtuinventory

## Quick commands

**Build & verify:**
- `cargo build` – compile and catch errors
- `cargo run` – run the TUI
- `cargo test` – runs 0 tests (no committed tests)
- `cargo fmt` – format code
- `cargo fmt -- --check` – formatting check

## Architecture

**Single-binary TUI with two screens:**
- `src/main.rs` owns terminal lifecycle, event loop, focus routing, and rendering
- `App` struct holds: `Vec<Item>`, table/list state, input fields, status, edit/delete mode, `Connection`, `Settings`, `Screen`, settings state
- Startup: `App::new()` → `Settings::load()` → `connect_db(path)` → schema setup → `get_items()`
- Screens: `Inventory` (default) and `Settings`
- Keyboard: `Ctrl+S` toggles screens, `Esc` closes settings, `q` quits

**Persistence (`src/db/`):**
- `sql.rs` – SQL constants (CREATE, SELECT, INSERT, DELETE)
- `db.rs` – `Item` types, `connect_db(path)`, `get_items()`, `try_create_item()`, `try_delete_item()`
- `mod.rs` – module wiring only

**Settings (`src/settings.rs`):**
- `Settings::load()` reads `app-settings.toml` or returns defaults
- `set_database_path()` updates path, maintains recent list (max 10), saves to file
- Default database: `inventory.sqlite3`
- TOML format: simple, human-editable

**No service layer:** Add/edit/delete write to SQLite and sync `app.items` in same handler

## Critical rules

**Terminal lifecycle (unbalanced → bad state):**
- `enable_raw_mode` ↔ `disable_raw_mode`
- `EnterAlternateScreen` ↔ `LeaveAlternateScreen`
- `show_cursor` on exit
- If unbalanced, terminal left in bad state

**Database:**
- Always use `connect_db(path)` with parameter (not hardcoded)
- Always use `get_items()`, `try_create_item()`, `try_delete_item()`
- Schema compatibility via `PRAGMA table_info(...)` inside `connect_db()`
- Never duplicate schema setup

**Screen switching:**
- Use `Screen` enum for routing
- `draw_ui()` dispatches to `draw_inventory_ui()` or `draw_settings_ui()`
- `handle_inventory_input()` and `handle_settings_input()` route keyboard

**Database switching:**
- `switch_database(path)` in settings handlers
- Closes old connection, updates settings, reconnects, reloads items, clears UI state
- Persists path to `app-settings.toml`

**UI state sync:**
- `app.items` is in-memory source of truth
- Mutate SQLite → update matching `app.items` entry in same handler
- Switching databases → wipe editing state, delete confirmations, input fields

## Input normalization

- Empty `barcode`, `serial`, `location` → `None`
- Quantity parsed as `u32`; blank = `0`

## Table selection

- `TableState::default()` and `TableState::select(Some(index))`
- Keep selection valid after add/delete (adjust index if needed)

## Focus & keyboard

**Inventory screen:**
- `Focus` enum routes input; `active_input_mut()` returns mutable ref
- Add control → update focus cycling, keyboard handling, visual styling

**Settings screen:**
- `SettingsFocus` enum routes to list or text field

## Edit & delete

- Edit mode: `editing_row` transforms button and pre-fills inputs
- Delete: two-step via `pending_delete` (request → confirm y/n)

## Git commit policy

- **Only commit/push if editing `.md` files**
- **Do NOT commit/push if editing code files** (`.rs`, `.toml`, `Cargo.lock`, etc.)
- User handles code changes manually; docs safe to auto-commit