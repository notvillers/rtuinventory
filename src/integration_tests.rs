// Integration tests for rtuinventory application

use std::fs;
use std::path::Path;
use tempfile::TempDir;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_connection() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let conn = rtuinventory::db::db::connect_db(db_path.to_str().unwrap());
        
        // Verify connection is valid and table exists
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM items", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_create_and_get_items() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let conn = rtuinventory::db::db::connect_db(db_path.to_str().unwrap());
        
        // Create an item
        let item_add = rtuinventory::db::db::try_create_item(
            &"Test Item".to_string(),
            &"".to_string(),
            &"".to_string(),
            &"".to_string(),
            &"5".to_string(),
            &conn
        );
        
        match item_add {
            rtuinventory::db::db::ItemAdd::Ok(item) => {
                assert_eq!(item.name, "Test Item");
                assert_eq!(item.quantity, 5);
            }
            rtuinventory::db::db::ItemAdd::Err(e) => panic!("Failed to create item: {}", e),
        }
        
        // Get all items
        let items = rtuinventory::db::db::get_items(&conn);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "Test Item");
        assert_eq!(items[0].quantity, 5);
    }
}