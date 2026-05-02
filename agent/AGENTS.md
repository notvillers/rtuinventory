# AGENTS guide for rtuinventory

## Purpose
Quick, actionable guidance so that agents will not miss any important repository details.

## Project Overview
- Rust CLI TUI powered by `ratatui` and `crossterm`.
- SQLite database `inventory.sqlite3` resides in the repository root.
- No tests exist; `cargo test` will succeed immediately.

## Key files
- `src/main.rs` – TUI entry point, event loop, rendering.
- `src/db/db.rs` – Schema helper, `connect_db`, CRUD helpers, and the `Item` model.
- `src/db/sql.rs` – SQL string constants.
- `Cargo.toml` – Dependencies: `crossterm = 0.28`, `ratatui = 0.29`, `rusqlite = { version = 0.29, features = ["bundled"] }`.

## Build & Run
```
cargo build
cargo run
```
The binary creates/opens `inventory.sqlite3` automatically; deleting the file resets the data.

## Common developer tasks
- **Add a package / file**: run `cargo build` to confirm no compilation errors. If you wish to format, run `cargo fmt`.
- **Edit UI layout**: keep terminal initialization (`enable_raw_mode`, `EnterAlternateScreen`, `show_cursor`) intact; modifying `src/main.rs` may break control flow.
- **Database schema changes**: always use functions in `src/db/db.rs`; do not edit `CREATE_SQL` directly unless you intend a migration.

## Verification
1. `cargo build` – ensures syntax and dependencies.
2. `cargo run` – test that the TUI behaves as expected.
3. `cargo fmt` (if rustfmt is installed) – keep code style consistent.

## Repository‑specific conventions
- No CI workflows or pre‑commit hooks are defined.
- No tests; all changes are verified through the binary.
- No external code generation or migration tools.

## Contact / PR guidelines
- Keep commits focused and small.
- Describe the intent in the PR title and body.
