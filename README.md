# rtuinventory

A simple terminal-based inventory management application built with Rust.

## Features

- SQLite database backed inventory tracking
- Terminal User Interface (TUI) using ratatui
- Add, edit, and delete inventory items
- Switch between different database files
- Settings persistence using TOML configuration

## Quick Start

1. Build the application:
   ```bash
   cargo build
   ```

2. Run the application:
   ```bash
   cargo run
   ```

## Architecture

- Single-binary TUI application
- Two screens: Inventory view and Settings
- SQLite database persistence
- Settings stored in `app-settings.toml`

## Controls

- `Ctrl+S` - Toggle between screens
- `Esc` - Close settings screen
- `q` - Quit application

## Database

- Default database: `inventory.sqlite3`
- Supports switching between different database files
- Automatic schema migration for older databases

## Development

- Build: `cargo build`
- Run: `cargo run`
- Format: `cargo fmt`
- Check formatting: `cargo fmt -- --check`