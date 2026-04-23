# GitHub Copilot Instructions

Purpose: brief, actionable guidance for GitHub Copilot and other coding agents to be immediately productive in this repository.

- Project type: Rust command-line TUI using `cargo` and `ratatui` (see `Cargo.toml`).
- Build: `cargo build`
- Run locally: `cargo run`
- Tests: none included; run `cargo test` if tests are added.

Key files:
- `src/main.rs` — TUI entrypoint and app logic (terminal setup/teardown, event loop, UI drawing).
- `src/db/db.rs` — SQLite helpers and `Item` model; use `connect_db()` and `get_items()` for DB access.
- `Cargo.toml` — dependencies and crate metadata.

Conventions and guidelines:
- Make minimal, focused changes; prefer small, testable commits.
- Preserve public APIs and file structure unless explicitly asked to refactor.
- When editing UI layout in `src/main.rs`, keep terminal setup/teardown intact (`enable_raw_mode`, alternate screen, `show_cursor` on exit).
- For database changes, reuse `connect_db()` and avoid duplicating schema setup logic.
- Use `TableState::default()` for table widgets and initialize selection where appropriate.

How to verify changes quickly:
- Build: `cargo build` to catch compile errors.
- Run: `cargo run` to manually verify TUI behavior.
- Format: `cargo fmt` (if installed) to keep style consistent.

If adding files or docs, link to existing docs rather than duplicating information.

If you need more specific agent behavior (e.g., automated tests, CI workflow, or granular refactor rules), request creating a specialized `AGENT` skill or a `.github` workflow.
