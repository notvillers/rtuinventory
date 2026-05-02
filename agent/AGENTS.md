# AGENTS guide for rtuinventory

## Quick reference

**Build & test:**
- `cargo build` – compile and catch errors
- `cargo run` – run the TUI
- `cargo test` – no committed tests; runs 0 tests
- `cargo fmt` – format code
- `cargo fmt -- --check` – formatting check only

## Architecture at a glance

**Single-binary TUI architecture:**
- `src/main.rs` owns terminal lifecycle (`enable_raw_mode`, `EnterAlternateScreen`, etc.), the event loop, focus routing, and all widget rendering.
- `App` struct holds runtime state: `Vec<Item>`, table selection, input fields, status, edit/delete mode, and the live SQLite connection.
- Startup: `App::new()` → `connect_db()` → opens `inventory.sqlite3` → schema setup → `get_items()` loads all items into memory.

**Persistence layer (`src/db/`):**
- `sql.rs` – SQL constants (CREATE, SELECT, INSERT, DELETE).
- `db.rs` – `Item` and `InsertItem` types, `connect_db()`, `get_items()`, `try_create_item()`, `try_delete_item()`.
- `mod.rs` – module wiring only.

**No service layer:** Add/edit/delete operations write to SQLite and sync `app.items` in the same handler. The in-memory vector is the source of truth for rendering.

**Layout:** Single table (top, flexible) + bottom control area (name/qty inputs, location input, barcode/serial inputs, buttons, status/help).

## When making changes

**Terminal lifecycle (critical):**
- `enable_raw_mode` ↔ `disable_raw_mode`
- `EnterAlternateScreen` ↔ `LeaveAlternateScreen`
- `show_cursor` on exit
- If these become unbalanced, the terminal is left in a bad state.

**Database changes:**
- Always use `connect_db()`, `get_items()`, `try_create_item()`, `try_delete_item()`.
- Schema compatibility for older DBs is built into `connect_db()` via `PRAGMA table_info(...)`.
- Never duplicate schema setup logic.

**UI state sync:**
- `app.items` is the in-memory source of truth for rendering.
- If you mutate SQLite, update the corresponding `app.items` entry in the same handler.

**Input normalization:**
- Empty `barcode`, `serial`, `location` → `None`
- Quantity parsed as `u32`; blank quantity = `0`

**Table selection:**
- Use `TableState::default()` and `TableState::select(Some(index))`.
- Keep selection valid after add/delete (adjust index if needed).

**Focus and keyboard:**
- `Focus` enum routes keyboard input to the active widget.
- `active_input_mut()` returns mutable ref to the focused input field.
- When adding a new interactive control, update:
  1. Focus cycling (`cycle_focus()`, `cycle_focus_back()`)
  2. Enter-key routing in `run_app()`
  3. Visual focus styling in `draw_ui()`

**Edit and delete:**
- Edit mode: `editing_row` field transforms "Add Item" button into "Save" and pre-fills input fields.
- Delete mode: Two-step flow via `pending_delete` (request → confirm y/n).

## Notes
- No CI workflows or pre-commit hooks.
- All changes verified through `cargo build` and `cargo run`.
- No external code generation or migration tools.