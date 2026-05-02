# GitHub Copilot Instructions

## Build, test, and format commands

- Build: `cargo build`
- Run the TUI: `cargo run`
- Full test suite: `cargo test` (the repository currently has no committed tests, so this runs 0 tests)
- Single test: `cargo test <test_name>` once named tests exist
- Format: `cargo fmt`
- Formatting check: `cargo fmt -- --check`

## High-level architecture

- This is a single-binary Rust TUI. `src/main.rs` owns terminal setup/teardown, the event loop, keyboard routing, and all widget rendering.
- `App` in `src/main.rs` is the central runtime state. It holds the loaded `Vec<Item>`, the selected table row, current input values, status text, edit/delete state, live SQLite `Connection`, and settings state.
- Startup flows through `App::new()`: it calls `Settings::load()` to read `app-settings.toml`, then `connect_db(path)` with the configured database path, ensures schema exists, and loads all items into memory with `get_items()`.
- Two screens controlled by `Screen` enum: `Inventory` (default, shows table/inputs) and `Settings` (database picker).
- Persistence is split under `src/db/`:
  - `src/db/sql.rs` contains the SQL constants.
  - `src/db/db.rs` contains the `Item`/`InsertItem` types, `connect_db(path)`, and CRUD helpers.
  - `src/db/mod.rs` only wires the modules together.
- Configuration in `src/settings.rs`: `Settings` struct holds the current database path and recent list. Serialized to `app-settings.toml` (TOML format). Defaults to `inventory.sqlite3` if missing.
- There is no separate service layer. Add, edit, and delete actions write directly to SQLite and then update `app.items` in the same handler so the UI stays in sync with the database.
- The inventory layout is fixed around one main table plus a bottom control area: name/quantity inputs, location input, barcode/serial inputs, action buttons, and a status/help line.
- The settings layout splits the screen: recent databases list (top) and new database path input (bottom).

## Key conventions

- Keep the terminal lifecycle in `main()` intact: `enable_raw_mode`, `EnterAlternateScreen`, `disable_raw_mode`, `LeaveAlternateScreen`, and `show_cursor` must stay paired or the terminal will be left in a bad state.
- Reuse `connect_db(path)` with a database path parameter (not hardcoded). `get_items()`, `try_create_item()`, and `try_delete_item()` for database work. Schema compatibility for older databases is handled inside `connect_db()` with `PRAGMA table_info(...)` checks.
- Runtime rendering uses `app.items` as the in-memory source of truth. If a change mutates the database, update the matching in-memory row/vector state in the same flow.
- Database switching via `switch_database()` closes the old connection, updates settings, reconnects, reloads items, and clears all UI state (editing, delete confirmation, input fields).
- Empty `barcode`, `serial`, and `location` inputs are normalized to `None`; quantity is parsed as `u32`, and a blank quantity means `0`.
- Table selection is stateful through `TableState`. Initialize it with `TableState::default()` and keep the selection valid after add/delete operations.
- Keyboard behavior on inventory screen is driven by the `Focus` enum plus `active_input_mut()`. If you add a new interactive control, update focus cycling, enter-key behavior, and visual focus styling together.
- Settings screen uses `SettingsFocus` enum to route keyboard input to the recent list or new database input field.
- Edit mode is controlled by `editing_row`, which turns the add button into save. Delete is intentionally two-step through `pending_delete`.
- Global shortcuts: `Ctrl+S` toggles between screens, `Esc` closes settings, `q` quits the app.

## Git commit policy

- **Only commit and push if you edit `.md` files (documentation).**
- If your changes include any code files (`.rs`, `.toml`, `Cargo.lock`, etc.), **do not commit or push**—leave them for the user to review and commit manually.
- This keeps the repository clean and gives the user full control over code changes, while documentation updates are safe to auto-commit.
