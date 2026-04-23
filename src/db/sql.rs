// SQLs
pub const DB_NAME: &'static str = "inventory.db";

pub const CREATE_SQL: &'static str = r#"
    CREATE TABLE IF NOT EXISTS items (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT NOT NULL,
        barcode TEXT,
        serial TEXT
    )
"#;

pub const SELECT_ITEMS: &'static str = r#"
    SELECT
        id,
        name,
        barcode,
        serial
    FROM
        items
    ORDER BY
        id ASC
"#;

pub const INSERT_ITEM: &'static str = r#"
    INSERT INTO
        items
            (name, barcode, serial)
        VALUES
            (?1, ?2, ?3)
"#;

//pub const 

pub const DELETE_ITEM: &'static str = r#"
    DELETE FROM
        items
    WHERE
        id = ?1
"#;
