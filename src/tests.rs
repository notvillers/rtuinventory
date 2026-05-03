// Unit tests for the rtuinventory application

use std::fs;
use std::path::Path;
use tempfile::TempDir;

mod db {
    use super::*;
    use rtuinventory::db::db::{connect_db, get_items, try_create_item, try_delete_item, ItemAdd, InsertItem};

    #[test]
    fn test_database_connection() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let conn = connect_db(db_path.to_str().unwrap());
        
        // Verify connection is valid and table exists
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM items", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_create_and_get_items() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let conn = connect_db(db_path.to_str().unwrap());
        
        // Create an item
        let item_add = try_create_item(
            &"Test Item".to_string(),
            &"".to_string(),
            &"".to_string(),
            &"".to_string(),
            &"5".to_string(),
            &conn
        );
        
        match item_add {
            ItemAdd::Ok(item) => {
                assert_eq!(item.name, "Test Item");
                assert_eq!(item.quantity, 5);
            }
            ItemAdd::Err(e) => panic!("Failed to create item: {}", e),
        }
        
        // Get all items
        let items = get_items(&conn);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "Test Item");
        assert_eq!(items[0].quantity, 5);
    }

    #[test]
    fn test_delete_item() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let conn = connect_db(db_path.to_str().unwrap());
        
        // Create an item
        let item_add = try_create_item(
            &"Test Item".to_string(),
            &"".to_string(),
            &"".to_string(),
            &"".to_string(),
            &"5".to_string(),
            &conn
        );
        
        let item = match item_add {
            ItemAdd::Ok(item) => item,
            ItemAdd::Err(e) => panic!("Failed to create item: {}", e),
        };
        
        // Delete the item
        try_delete_item(&item.id, &conn);
        
        // Verify item was deleted
        let items = get_items(&conn);
        assert_eq!(items.len(), 0);
    }

    #[test]
    fn test_item_with_optional_fields() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let conn = connect_db(db_path.to_str().unwrap());
        
        // Create an item with optional fields
        let item_add = try_create_item(
            &"Test Item".to_string(),
            &"123456".to_string(),
            &"ABC123".to_string(),
            &"Warehouse A".to_string(),
            &"10".to_string(),
            &conn
        );
        
        match item_add {
            ItemAdd::Ok(item) => {
                assert_eq!(item.name, "Test Item");
                assert_eq!(item.barcode, Some("123456".to_string()));
                assert_eq!(item.serial, Some("ABC123".to_string()));
                assert_eq!(item.location, Some("Warehouse A".to_string()));
                assert_eq!(item.quantity, 10);
            }
            ItemAdd::Err(e) => panic!("Failed to create item: {}", e),
        }
    }

    #[test]
    fn test_create_item_with_empty_fields() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let conn = connect_db(db_path.to_str().unwrap());
        
        // Test creating item with empty name should fail
        let item_add = try_create_item(
            &"".to_string(),  // Empty name
            &"".to_string(),
            &"".to_string(),
            &"".to_string(),
            &"5".to_string(),
            &conn
        );
        
        match item_add {
            ItemAdd::Err(_) => {
                // Expected - empty name should cause error
            }
            ItemAdd::Ok(_) => panic!("Should have failed with empty name"),
        }
    }

    #[test]
    fn test_create_item_with_invalid_quantity() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let conn = connect_db(db_path.to_str().unwrap());
        
        // Test creating item with invalid quantity should fail
        let item_add = try_create_item(
            &"Test Item".to_string(),
            &"".to_string(),
            &"".to_string(),
            &"".to_string(),
            &"invalid".to_string(),  // Invalid quantity
            &conn
        );
        
        match item_add {
            ItemAdd::Err(_) => {
                // Expected - invalid quantity should cause error
            }
            ItemAdd::Ok(_) => panic!("Should have failed with invalid quantity"),
        }
    }
}

mod settings {
    use super::*;
    use rtuinventory::settings::Settings;

    #[test]
    fn test_settings_load_default() {
        // Test that Settings::load() returns default settings when file doesn't exist
        let settings = Settings::load();
        assert_eq!(settings.database.path, "inventory.sqlite3");
        assert_eq!(settings.database.recent.len(), 1);
        assert_eq!(settings.database.recent[0], "inventory.sqlite3");
    }

    #[test]
    fn test_settings_set_database_path() {
        let mut settings = Settings::default();
        settings.set_database_path("test.db".to_string());
        
        assert_eq!(settings.database.path, "test.db");
        assert_eq!(settings.database.recent.len(), 2);
        assert_eq!(settings.database.recent[0], "test.db");
    }

    #[test]
    fn test_settings_recent_paths_limit() {
        let mut settings = Settings::default();
        // Add more than 10 paths to test limit
        for i in 0..15 {
            settings.set_database_path(format!("db{}.db", i));
        }
        
        assert_eq!(settings.database.recent.len(), 10);
        // Most recent should be first
        assert_eq!(settings.database.recent[0], "db14.db");
    }
}