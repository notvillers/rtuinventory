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
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
};

use rusqlite::{Connection, params};

mod db;

use crate::db::{
    sql::DB_NAME,
    db::{
        Item, ItemAdd,
        connect_db,
        get_items, try_create_item, try_delete_item
    }
};

// Which widget currently receives keyboard input.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Focus {
    Table,
    NameInput,
    QuantityInput,
    BarcodeInput,
    SerialInput,
    ButtonAdd,
    ButtonEdit,
    ButtonDelete,
}

// All mutable application state used by the event loop and renderer.
struct App {
    items: Vec<Item>,
    table_state: TableState,
    focus: Focus,
    name_input: String,
    quantity_input: String,
    barcode_input: String,
    serial_input: String,
    status: String,
    conn: Connection,
    editing_row: Option<usize>,
    pending_delete: Option<usize>,
}

impl App {
    // Seed the demo with sample rows and initial focus/state.
    fn new() -> Self {
        // Create/open the SQLite DB and ensure the items table exists.
        let conn = connect_db();

        // Load all rows from DB into memory.
        let items = get_items(&conn);
        // Initialize table state and select first row if present.
        let mut table_state = TableState::default();
        if !items.is_empty() {
            table_state.select(Some(0));
        }

        Self {
            items,
            table_state,
            focus: Focus::NameInput,
            name_input: String::new(),
            quantity_input: String::new(),
            barcode_input: String::new(),
            serial_input: String::new(),
            status: format!("Loaded from {0}", DB_NAME).to_string(),
            conn,
            editing_row: None,
            pending_delete: None,
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

    // Rotate focus order: table -> input -> button -> table.
    fn cycle_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Table => Focus::NameInput,
            Focus::NameInput => Focus::QuantityInput,
            Focus::QuantityInput => Focus::BarcodeInput,
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
            Focus::BarcodeInput => Focus::QuantityInput,
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
                // Update DB
                self.conn
                    .execute(
                        "UPDATE items SET name = ?1, barcode = ?2, serial = ?3, quantity = ?4 WHERE id = ?5",
                        params![name, barcode, serial, qty_val as i64, id],
                    )
                    .expect("update failed");
                self.items[i] = Item {
                    id,
                    name: name.to_string(),
                    barcode,
                    serial,
                    quantity: qty_val,
                };
                self.status = "Item updated".to_string();
                self.editing_row = None;
                self.name_input.clear();
                self.quantity_input.clear();
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
    let mut app = App::new();

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
                continue;
            }

            match key.code {
                KeyCode::Char('q') => return Ok(()),
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
    }
}

// Render full UI: table, input, button, and status/help line.
fn draw_ui(frame: &mut Frame, app: &mut App) {
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

    let header = Row::new(vec!["ID", "Name", "Qty", "Barcode", "Serial"]).style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );

    let rows = app.items.iter().map(|item| {
        Row::new(vec![
            Cell::from(item.id.to_string()),
            Cell::from(item.name.clone()),
            Cell::from(item.quantity.to_string()),
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
            Constraint::Percentage(32),
            Constraint::Percentage(32),
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
        "{} | Focus: {:?} | Tab/Enter: next focus | Up/Down: move row | q: quit",
        app.status, app.focus
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
    frame.render_widget(barcode_input, bottom_chunks[1]);
    frame.render_widget(serial_input, bottom_chunks[2]);
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
