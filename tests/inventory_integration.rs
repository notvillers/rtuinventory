use std::env;
use std::sync::Mutex;

use tempfile::TempDir;

use rtuinventory::db::db::{
    connect_db, get_items, trim_or_none_value, try_create_item, try_delete_item, ItemAdd,
};
use rtuinventory::settings::Settings;

static SETTINGS_FILE_LOCK: Mutex<()> = Mutex::new(());

fn setup_db() -> (TempDir, rusqlite::Connection) {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let db_path = temp_dir.path().join("test.sqlite3");
    let conn = connect_db(db_path.to_str().expect("temp db path is not valid UTF-8"));
    (temp_dir, conn)
}

#[test]
fn connect_db_creates_items_table() {
    let (_temp_dir, conn) = setup_db();

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM items", [], |r| r.get(0))
        .expect("items table query should succeed");

    assert_eq!(count, 0);
}

#[test]
fn create_item_and_read_back() {
    let (_temp_dir, conn) = setup_db();

    let created = try_create_item(
        &"Test Item".to_string(),
        &"".to_string(),
        &"".to_string(),
        &"".to_string(),
        &"5".to_string(),
        &conn,
    );

    let item = match created {
        ItemAdd::Ok(item) => item,
        ItemAdd::Err(e) => panic!("item creation failed unexpectedly: {e}"),
    };

    assert_eq!(item.name, "Test Item");
    assert_eq!(item.quantity, 5);
    assert_eq!(item.barcode, None);
    assert_eq!(item.serial, None);
    assert_eq!(item.location, None);

    let items = get_items(&conn);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].name, "Test Item");
    assert_eq!(items[0].quantity, 5);
}

#[test]
fn delete_item_removes_row() {
    let (_temp_dir, conn) = setup_db();

    let created = try_create_item(
        &"Delete Me".to_string(),
        &"".to_string(),
        &"".to_string(),
        &"".to_string(),
        &"1".to_string(),
        &conn,
    );

    let item = match created {
        ItemAdd::Ok(item) => item,
        ItemAdd::Err(e) => panic!("item creation failed unexpectedly: {e}"),
    };

    try_delete_item(&item.id, &conn);
    let items = get_items(&conn);
    assert!(items.is_empty());
}

#[test]
fn create_item_with_optional_fields() {
    let (_temp_dir, conn) = setup_db();

    let created = try_create_item(
        &"Has Fields".to_string(),
        &"123456".to_string(),
        &"ABC123".to_string(),
        &"Warehouse A".to_string(),
        &"10".to_string(),
        &conn,
    );

    let item = match created {
        ItemAdd::Ok(item) => item,
        ItemAdd::Err(e) => panic!("item creation failed unexpectedly: {e}"),
    };

    assert_eq!(item.barcode.as_deref(), Some("123456"));
    assert_eq!(item.serial.as_deref(), Some("ABC123"));
    assert_eq!(item.location.as_deref(), Some("Warehouse A"));
    assert_eq!(item.quantity, 10);
}

#[test]
fn create_item_rejects_empty_name() {
    let (_temp_dir, conn) = setup_db();

    let created = try_create_item(
        &"".to_string(),
        &"".to_string(),
        &"".to_string(),
        &"".to_string(),
        &"5".to_string(),
        &conn,
    );

    match created {
        ItemAdd::Err(msg) => assert_eq!(msg, "Name is required"),
        ItemAdd::Ok(_) => panic!("expected empty name to fail"),
    }
}

#[test]
fn create_item_rejects_invalid_quantity() {
    let (_temp_dir, conn) = setup_db();

    let created = try_create_item(
        &"Test Item".to_string(),
        &"".to_string(),
        &"".to_string(),
        &"".to_string(),
        &"invalid".to_string(),
        &conn,
    );

    match created {
        ItemAdd::Err(msg) => assert_eq!(msg, "Quantity must be a non-negative number"),
        ItemAdd::Ok(_) => panic!("expected invalid quantity to fail"),
    }
}

#[test]
fn create_item_blank_quantity_defaults_to_zero() {
    let (_temp_dir, conn) = setup_db();

    let created = try_create_item(
        &"Zero Qty".to_string(),
        &"".to_string(),
        &"".to_string(),
        &"".to_string(),
        &"   ".to_string(),
        &conn,
    );

    let item = match created {
        ItemAdd::Ok(item) => item,
        ItemAdd::Err(e) => panic!("item creation failed unexpectedly: {e}"),
    };

    assert_eq!(item.quantity, 0);
}

#[test]
fn trim_or_none_value_trims_and_handles_empty() {
    assert_eq!(trim_or_none_value(&"  value  ".to_string()), Some("value".to_string()));
    assert_eq!(trim_or_none_value(&"   ".to_string()), None);
}

#[test]
fn settings_default_and_recent_paths_behavior() {
    let _guard = SETTINGS_FILE_LOCK
        .lock()
        .expect("failed to lock settings file guard");

    let original_cwd = env::current_dir().expect("failed to get current directory");
    let temp_dir = TempDir::new().expect("failed to create temp dir");

    env::set_current_dir(temp_dir.path()).expect("failed to switch to temp directory");

    // Keep this deterministic by controlling the directory where app-settings.toml lives.
    let mut settings = Settings::load();
    assert_eq!(settings.get_database_path(), "inventory.sqlite3");
    assert_eq!(settings.database.recent, vec!["inventory.sqlite3".to_string()]);

    settings.set_database_path("db1.sqlite3".to_string());
    settings.set_database_path("db2.sqlite3".to_string());
    settings.set_database_path("db1.sqlite3".to_string());

    assert_eq!(settings.get_database_path(), "db1.sqlite3");
    assert_eq!(
        settings.database.recent,
        vec![
            "db1.sqlite3".to_string(),
            "db2.sqlite3".to_string(),
            "inventory.sqlite3".to_string()
        ]
    );

    env::set_current_dir(original_cwd).expect("failed to restore current directory");
}
