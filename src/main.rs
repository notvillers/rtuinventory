use std::io;
use std::time::Duration;

use crossterm::{
    execute,
    event::{
        self,
        Event, KeyCode, KeyEventKind, KeyModifiers
    },
    terminal::{
        EnterAlternateScreen, LeaveAlternateScreen,
        disable_raw_mode, enable_raw_mode
    }
};

use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Cell, List, ListItem, ListState, Paragraph, Row, Table, TableState},
};

use rusqlite::{Connection, params};

mod db;
mod settings;

use crate::db::db::{
    Item, ItemAdd,
    connect_db,
    get_items, try_create_item, try_delete_item
};
use crate::settings::Settings;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Screen {
    Inventory,
    Settings,
}

// Which widget currently receives keyboard input.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Focus {
    Table,
    NameInput,
    QuantityInput,
    LocationInput,
    BarcodeInput,
    SerialInput,
    ButtonAdd,
    ButtonEdit,
    ButtonDelete,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum SettingsFocus {
    DatabaseList,
    NewDatabaseInput,
}

// All mutable application state used by the event loop and renderer.
struct App {
    items: Vec<Item>,
    table_state: TableState,
    focus: Focus,
    name_input: String,
    quantity_input: String,
    location_input: String,
    barcode_input: String,
    serial_input: String,
    status: String,
    conn: Connection,
    editing_row: Option<usize>,
    pending_delete: Option<usize>,
    screen: Screen,
    settings: Settings,
    settings_focus: SettingsFocus,
    settings_list_state: ListState,
    new_db_input: String,
}

impl App {
    // Seed the demo with sample rows and initial focus/state.
    fn new(settings: Settings) -> Self {
        // Create/open the SQLite DB and ensure the items table exists.
        let db_path = settings.get_database_path().to_string();
        let conn = connect_db(&db_path);

        // Load all rows from DB into memory.
        let items = get_items(&conn);
        // Initialize table state and select first row if present.
        let mut table_state = TableState::default();
        if !items.is_empty() {
            table_state.select(Some(0));
        }

        let mut settings_list_state = ListState::default();
        settings_list_state.select(Some(0));

        Self {
            items,
            table_state,
            focus: Focus::NameInput,
            name_input: String::new(),
            quantity_input: String::new(),
            location_input: String::new(),
            barcode_input: String::new(),
            serial_input: String::new(),
            status: format!("Loaded from {}", db_path),
            conn,
            editing_row: None,
            pending_delete: None,
            screen: Screen::Inventory,
            settings,
            settings_focus: SettingsFocus::DatabaseList,
            settings_list_state,
            new_db_input: String::new(),
        }
    }

    // Move table selection down and wrap at the end.
    fn next_row(&mut self) {
        if self.items.is_empty() {
            self.table_state.select(None);
            return;
        }

        let i = self.table_state.selected().unwrap_or(0);
        let next = if i >= self.items.len().saturating_sub(1) {
            0
        } else {
            i + 1
        };
        self.table_state.select(Some(next));
    }

    // Move table selection up and wrap at the beginning.
    fn prev_row(&mut self) {
        if self.items.is_empty() {
            self.table_state.select(None);
            return;
        }

        let i = self.table_state.selected().unwrap_or(0);
        let prev = if i == 0 { self.items.len() - 1 } else { i - 1 };
        self.table_state.select(Some(prev));
    }

    fn switch_database(&mut self, db_path: String) {
        // Update settings and reconnect
        self.settings.set_database_path(db_path.clone());
        self.conn = connect_db(&db_path);

        // Reload items
        self.items = get_items(&self.conn);
        self.table_state.select(if self.items.is_empty() { None } else { Some(0) });
        self.status = format!("Switched to: {}", db_path);

        // Clear editing state
        self.editing_row = None;
        self.pending_delete = None;
        self.name_input.clear();
        self.quantity_input.clear();
        self.location_input.clear();
        self.barcode_input.clear();
        self.serial_input.clear();
    }

    fn cycle_settings_focus(&mut self) {
        self.settings_focus = match self.settings_focus {
            SettingsFocus::DatabaseList => SettingsFocus::NewDatabaseInput,
            SettingsFocus::NewDatabaseInput => SettingsFocus::DatabaseList,
        };
    }

    fn settings_next(&mut self) {
        let len = self.settings.database.recent.len();
        if len == 0 {
            return;
        }
        let current = self.settings_list_state.selected().unwrap_or(0);
        let next = if current >= len - 1 { 0 } else { current + 1 };
        self.settings_list_state.select(Some(next));
    }

    fn settings_prev(&mut self) {
        let len = self.settings.database.recent.len();
        if len == 0 {
            return;
        }
        let current = self.settings_list_state.selected().unwrap_or(0);
        let prev = if current == 0 { len - 1 } else { current - 1 };
        self.settings_list_state.select(Some(prev));
    }

    // Rotate focus order: table -> input -> button -> table.
    fn cycle_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Table => Focus::NameInput,
            Focus::NameInput => Focus::QuantityInput,
            Focus::QuantityInput => Focus::LocationInput,
            Focus::LocationInput => Focus::BarcodeInput,
            Focus::BarcodeInput => Focus::SerialInput,
            Focus::SerialInput => Focus::ButtonAdd,
            Focus::ButtonAdd => Focus::ButtonEdit,
            Focus::ButtonEdit => Focus::ButtonDelete,
            Focus::ButtonDelete => Focus::Table,
        };
    }

    // Rotate focus backward (reverse of cycle_focus).
    fn cycle_focus_back(&mut self) {
        self.focus = match self.focus {
            Focus::Table => Focus::ButtonDelete,
            Focus::NameInput => Focus::Table,
            Focus::QuantityInput => Focus::NameInput,
            Focus::LocationInput => Focus::QuantityInput,
            Focus::BarcodeInput => Focus::LocationInput,
            Focus::SerialInput => Focus::BarcodeInput,
            Focus::ButtonAdd => Focus::SerialInput,
            Focus::ButtonEdit => Focus::ButtonAdd,
            Focus::ButtonDelete => Focus::ButtonEdit,
        };
    }

    fn active_input_mut(&mut self) -> Option<&mut String> {
        match self.focus {
            Focus::NameInput => Some(&mut self.name_input),
            Focus::QuantityInput => Some(&mut self.quantity_input),
            Focus::LocationInput => Some(&mut self.location_input),
            Focus::BarcodeInput => Some(&mut self.barcode_input),
            Focus::SerialInput => Some(&mut self.serial_input),
            _ => None,
        }
    }

    // Build a new item from the three fields and append it to the table.
    fn try_add_item(&mut self) {
        let item_add = match try_create_item(
            &self.name_input,
            &self.barcode_input,
            &self.serial_input,
            &self.location_input,
            &self.quantity_input,
            &self.conn
        ) {
            ItemAdd::Ok(item) => item,
            ItemAdd::Err(error) => {
                self.status = error;
                return;
            }
        };
        self.items.push(item_add);
        self.name_input.clear();
        self.quantity_input.clear();
        self.location_input.clear();
        self.barcode_input.clear();
        self.serial_input.clear();
        self.status = "Item added".to_string();
        self.table_state.select(Some(self.items.len() - 1));
        self.focus = Focus::NameInput;
    }

    // Start editing the currently selected row (load values into inputs).
    fn start_edit(&mut self) {
        if let Some(i) = self.table_state.selected() {
            if i < self.items.len() {
                let it = &self.items[i];
                self.name_input = it.name.clone();
                self.quantity_input = it.quantity.to_string();
                    self.location_input = it.location.clone().unwrap_or_default();
                self.barcode_input = it.barcode.clone().unwrap_or_default();
                self.serial_input = it.serial.clone().unwrap_or_default();
                self.editing_row = Some(i);
                self.status = format!("Editing row {} - modify fields then press Save", it.id);
                self.focus = Focus::NameInput;
            } else {
                self.status = "No row selected".to_string();
            }
        } else {
            self.status = "No row selected".to_string();
        }
    }

    // Save edits back to the selected row if one is being edited.
    fn save_edit(&mut self) {
        if let Some(i) = self.editing_row {
            if i < self.items.len() {
                let name = self.name_input.trim();
                if name.is_empty() {
                    self.status = "Name is required".to_string();
                    return;
                }
                let barcode = if self.barcode_input.trim().is_empty() {
                    None
                } else {
                    Some(self.barcode_input.trim().to_string())
                };
                let serial = if self.serial_input.trim().is_empty() {
                    None
                } else {
                    Some(self.serial_input.trim().to_string())
                };
                let location = if self.location_input.trim().is_empty() {
                    None
                } else {
                    Some(self.location_input.trim().to_string())
                };
                let id = self.items[i].id;
                let qty_trim = self.quantity_input.trim();
                let qty_val: u32 = if qty_trim.is_empty() {
                    0
                } else {
                    match qty_trim.parse::<u32>() {
                        Ok(v) => v,
                        Err(_) => { self.status = "Quantity must be a non-negative number".to_string(); return; }
                    }
                };
                // Update DB (use location from input)
                self.conn
                    .execute(
                        "UPDATE items SET name = ?1, barcode = ?2, serial = ?3, location = ?4, quantity = ?5 WHERE id = ?6",
                        params![name, barcode, serial, location, qty_val as i64, id],
                    )
                    .expect("update failed");
                self.items[i] = Item {
                    id,
                    name: name.to_string(),
                    barcode,
                    serial,
                    location,
                    quantity: qty_val,
                };
                self.status = "Item updated".to_string();
                self.editing_row = None;
                self.name_input.clear();
                self.quantity_input.clear();
                self.location_input.clear();
                self.barcode_input.clear();
                self.serial_input.clear();
                self.focus = Focus::NameInput;
            }
        } else {
            self.status = "Not editing any row".to_string();
        }
    }

    // Request confirmation to delete the selected row.
    fn request_delete(&mut self) {
        if let Some(i) = self.table_state.selected() {
            if i < self.items.len() {
                self.pending_delete = Some(i);
                self.status = format!("Confirm delete of ID {}? (y/n)", self.items[i].id);
            } else {
                self.status = "No row selected".to_string();
            }
        } else {
            self.status = "No row selected".to_string();
        }
    }

    // Perform delete (called after confirmation).
    fn confirm_delete(&mut self) {
        if let Some(i) = self.pending_delete {
            if i < self.items.len() {
                let removed = self.items.remove(i);
                // delete from DB
                try_delete_item(&removed.id, &self.conn);
                self.status = format!("Deleted ID {}", removed.id);
                self.pending_delete = None;
                // adjust selection
                if self.items.is_empty() {
                    self.table_state.select(None);
                } else {
                    let sel = if i == 0 { 0 } else { i.saturating_sub(1) };
                    self.table_state.select(Some(sel));
                }
            }
        }
    }
}

fn handle_inventory_input(app: &mut App, key: crossterm::event::KeyEvent) {
    // Route keys based on focused control. Handle delete confirmation separately.
    if let Some(_) = app.pending_delete {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                app.confirm_delete();
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                app.pending_delete = None;
                app.status = "Delete canceled".to_string();
            }
            _ => {}
        }
        return;
    }

    match key.code {
        KeyCode::Tab => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                app.cycle_focus_back();
            } else {
                app.cycle_focus();
            }
        }
        KeyCode::BackTab => app.cycle_focus_back(),
        KeyCode::Up if app.focus == Focus::Table => app.prev_row(),
        KeyCode::Down if app.focus == Focus::Table => app.next_row(),
        KeyCode::Backspace => {
            if let Some(field) = app.active_input_mut() {
                field.pop();
            }
        }
        KeyCode::Enter => match app.focus {
            Focus::ButtonAdd => {
                // If we're editing, save; otherwise add new
                if app.editing_row.is_some() {
                    app.save_edit();
                } else {
                    app.try_add_item();
                }
            }
            Focus::ButtonEdit => app.start_edit(),
            Focus::ButtonDelete => app.request_delete(),
            _ => app.cycle_focus(),
        },
        KeyCode::Char(c) => {
            if let Some(field) = app.active_input_mut() {
                field.push(c);
            }
        }
        _ => {}
    }
}

fn handle_settings_input(app: &mut App, key: crossterm::event::KeyEvent) {
    match app.settings_focus {
        SettingsFocus::DatabaseList => match key.code {
            KeyCode::Up => app.settings_prev(),
            KeyCode::Down => app.settings_next(),
            KeyCode::Enter => {
                if let Some(idx) = app.settings_list_state.selected() {
                    if idx < app.settings.database.recent.len() {
                        let db_path = app.settings.database.recent[idx].clone();
                        app.switch_database(db_path);
                        app.screen = Screen::Inventory;
                    }
                }
            }
            KeyCode::Tab => app.cycle_settings_focus(),
            _ => {}
        },
        SettingsFocus::NewDatabaseInput => match key.code {
            KeyCode::Enter => {
                if !app.new_db_input.trim().is_empty() {
                    let db_path = app.new_db_input.trim().to_string();
                    app.new_db_input.clear();
                    app.switch_database(db_path);
                    app.screen = Screen::Inventory;
                }
            }
            KeyCode::Backspace => {
                app.new_db_input.pop();
            }
            KeyCode::Tab => app.cycle_settings_focus(),
            KeyCode::Char(c) => {
                app.new_db_input.push(c);
            }
            _ => {}
        },
    }
}

// Terminal setup/teardown wrapper for running the TUI safely.
fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

// Main app loop: draw every frame and handle keyboard events.
fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    let settings = Settings::load();
    let mut app = App::new(settings);

    loop {
        terminal.draw(|f| draw_ui(f, &mut app))?;

        if event::poll(Duration::from_millis(100))? {
            let Event::Key(key) = event::read()? else {
                continue;
            };

            // Ignore key repeat/release events so actions trigger once per keypress.
            if key.kind != KeyEventKind::Press {
                continue;
            }

            // Global keyboard shortcuts
            match key.code {
                KeyCode::Char('q') => return Ok(()),
                KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    app.screen = match app.screen {
                        Screen::Inventory => Screen::Settings,
                        Screen::Settings => Screen::Inventory,
                    };
                    if app.screen == Screen::Settings {
                        app.settings_list_state.select(Some(0));
                    }
                    continue;
                }
                KeyCode::Esc => {
                    if app.screen == Screen::Settings {
                        app.screen = Screen::Inventory;
                    }
                    continue;
                }
                _ => {}
            }

            // Route keys based on screen
            if app.screen == Screen::Settings {
                handle_settings_input(&mut app, key);
            } else {
                handle_inventory_input(&mut app, key);
            }
        }
    }
}

// Render full UI: table, input, button, and status/help line.
fn draw_ui(frame: &mut Frame, app: &mut App) {
    if app.screen == Screen::Settings {
        draw_settings_ui(frame, app);
    } else {
        draw_inventory_ui(frame, app);
    }
}

fn draw_settings_ui(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(frame.area());

    // Recent databases list
    let recent_items: Vec<ListItem> = app
        .settings
        .database
        .recent
        .iter()
        .enumerate()
        .map(|(_, db_path)| {
            let is_current = db_path == app.settings.get_database_path();
            let content = if is_current {
                format!("✓ {}", db_path)
            } else {
                db_path.clone()
            };
            ListItem::new(content)
        })
        .collect();

    let recent_list = List::new(recent_items)
        .block(Block::default().borders(Borders::ALL).title(
            if app.settings_focus == SettingsFocus::DatabaseList {
                "Recent Databases (active)"
            } else {
                "Recent Databases"
            }
        ))
        .style(Style::default())
        .highlight_style(
            Style::default()
                .bg(Color::Green)
                .add_modifier(Modifier::BOLD)
        )
        .highlight_symbol(" > ");

    let mut list_state = app.settings_list_state.clone();
    frame.render_stateful_widget(recent_list, chunks[0], &mut list_state);

    // New database input
    let new_db_block = Block::default()
        .borders(Borders::ALL)
        .title(
            if app.settings_focus == SettingsFocus::NewDatabaseInput {
                "New Database Path (active)"
            } else {
                "New Database Path"
            }
        );

    let new_db_block = if app.settings_focus == SettingsFocus::NewDatabaseInput {
        new_db_block.border_style(Style::default().fg(Color::Green))
    } else {
        new_db_block
    };

    let new_db_input = Paragraph::new(app.new_db_input.as_str()).block(new_db_block);
    frame.render_widget(new_db_input, chunks[1]);
}

fn draw_inventory_ui(frame: &mut Frame, app: &mut App) {
    // Layout: table takes the flexible top area; inputs/buttons/help are fixed at bottom.
    // Compute total height for the bottom fixed area: 3 inputs (3) + buttons (3) + help (2) = 14
    let bottom_fixed = 3 + 3 + 3 + 3 + 2; // name, barcode, serial, buttons row, help
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(bottom_fixed)])
        .split(frame.area());

    // Split the bottom fixed area into its rows (inputs, buttons, help)
    let bottom_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(2),
        ])
        .split(chunks[1]);

    let header = Row::new(vec!["ID", "Name", "Qty", "Location", "Barcode", "Serial"]).style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );

    let rows = app.items.iter().map(|item| {
        Row::new(vec![
            Cell::from(item.id.to_string()),
            Cell::from(item.name.clone()),
            Cell::from(item.quantity.to_string()),
            Cell::from(item.location.clone().unwrap_or_else(|| "-".to_string())),
            Cell::from(item.barcode.clone().unwrap_or_else(|| "-".to_string())),
            Cell::from(item.serial.clone().unwrap_or_else(|| "-".to_string())),
        ])
    });

    // Stateful table uses app.table_state to track selected row.
    // When table is focused, make highlight green to indicate active.
    let table_block = Block::default().borders(Borders::ALL).title("Inventory Grid");
    let table = Table::new(
        rows,
        [
            Constraint::Length(6),
            Constraint::Percentage(30),
            Constraint::Length(6),
            Constraint::Percentage(16),
            Constraint::Percentage(24),
            Constraint::Percentage(24),
        ],
    )
    .header(header)
    .block(table_block)
    .row_highlight_style(
        if app.focus == Focus::Table {
            Style::default().bg(Color::Green).add_modifier(Modifier::BOLD)
        } else {
            Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
        }
    )
    .highlight_symbol(" > ");

    // Input blocks: show green border when focused.
    let name_title = if app.focus == Focus::NameInput { "Name (active)" } else { "Name" };
    let mut name_block = Block::default().borders(Borders::ALL).title(name_title);
    if app.focus == Focus::NameInput {
        name_block = name_block.border_style(Style::default().fg(Color::Green));
    }
    let name_input = Paragraph::new(app.name_input.as_str()).block(name_block);

    let qty_title = if app.focus == Focus::QuantityInput { "Qty (active)" } else { "Qty" };
    let mut qty_block = Block::default().borders(Borders::ALL).title(qty_title);
    if app.focus == Focus::QuantityInput {
        qty_block = qty_block.border_style(Style::default().fg(Color::Green));
    }
    let qty_input = Paragraph::new(app.quantity_input.as_str()).block(qty_block);

    let barcode_title = if app.focus == Focus::BarcodeInput {
        "Barcode (active, optional)"
    } else {
        "Barcode (optional)"
    };
    let mut barcode_block = Block::default().borders(Borders::ALL).title(barcode_title);
    if app.focus == Focus::BarcodeInput {
        barcode_block = barcode_block.border_style(Style::default().fg(Color::Green));
    }
    let barcode_input = Paragraph::new(app.barcode_input.as_str()).block(barcode_block);

    let serial_title = if app.focus == Focus::SerialInput { "Serial (active, optional)" } else { "Serial (optional)" };
    let mut serial_block = Block::default().borders(Borders::ALL).title(serial_title);
    if app.focus == Focus::SerialInput {
        serial_block = serial_block.border_style(Style::default().fg(Color::Green));
    }
    let serial_input = Paragraph::new(app.serial_input.as_str()).block(serial_block);

    let location_title = if app.focus == Focus::LocationInput { "Location (active, optional)" } else { "Location (optional)" };
    let mut location_block = Block::default().borders(Borders::ALL).title(location_title);
    if app.focus == Focus::LocationInput {
        location_block = location_block.border_style(Style::default().fg(Color::Green));
    }
    let location_input = Paragraph::new(app.location_input.as_str()).block(location_block);

    // Button style changes when focused to simulate "active" state.
    // Buttons: Add/Save, Edit, Delete - each has its own focus state and style.
    let add_label = if app.editing_row.is_some() { "[ Save ]" } else { "[ Add Item ]" };
    let add_style = if app.focus == Focus::ButtonAdd {
        Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White).bg(Color::Blue)
    };
    let add_button = Paragraph::new(add_label).style(add_style).block(Block::default().borders(Borders::ALL).title("Add/Save"));

    let edit_style = if app.focus == Focus::ButtonEdit {
        Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White).bg(Color::Blue)
    };
    let edit_button = Paragraph::new("[ Edit ]").style(edit_style).block(Block::default().borders(Borders::ALL).title("Edit"));

    let del_style = if app.focus == Focus::ButtonDelete {
        Style::default().fg(Color::Black).bg(Color::Red).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White).bg(Color::DarkGray)
    };
    let del_button = Paragraph::new("[ Delete ]").style(del_style).block(Block::default().borders(Borders::ALL).title("Delete"));

    let help = Paragraph::new(Line::from(format!(
        "{} | Tab/Enter: next focus | Up/Down: move row | Ctrl+S: settings | q: quit",
        app.status
    )));

    // Paint each widget into its assigned region.
    // Render table in the flexible top chunk so it grows/shrinks with terminal height.
    frame.render_stateful_widget(table, chunks[0], &mut app.table_state);
    // Render bottom fixed rows from bottom_chunks.
    // Render name and quantity side-by-side on the first bottom row.
    let name_qty_row = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(80), Constraint::Length(8)])
        .split(bottom_chunks[0]);
    frame.render_widget(name_input, name_qty_row[0]);
    frame.render_widget(qty_input, name_qty_row[1]);
    frame.render_widget(location_input, bottom_chunks[1]);
    let barcode_serial_row = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(bottom_chunks[2]);
    frame.render_widget(barcode_input, barcode_serial_row[0]);
    frame.render_widget(serial_input, barcode_serial_row[1]);
    // render three buttons horizontally in the single bottom_chunks[3] area
    let button_row = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(33), Constraint::Percentage(33), Constraint::Percentage(34)])
        .split(bottom_chunks[3]);
    frame.render_widget(add_button, button_row[0]);
    frame.render_widget(edit_button, button_row[1]);
    frame.render_widget(del_button, button_row[2]);
    frame.render_widget(help, bottom_chunks[4]);
}
