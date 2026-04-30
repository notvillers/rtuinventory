// SQLs
pub const DB_NAME: &'static str = "inventory.db";

pub const CREATE_SQL: &'static str = r#"
    CREATE TABLE IF NOT EXISTS items (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT NOT NULL,
        barcode TEXT,
        serial TEXT,
        location TEXT,
        quantity INTEGER DEFAULT 0
    )
"#;

pub const SELECT_ITEMS: &'static str = r#"
    SELECT
        id,
        name,
        barcode,
        serial,
        location,
        quantity
    FROM
        items
    ORDER BY
        name COLLATE NOCASE ASC
"#;

pub const INSERT_ITEM: &'static str = r#"
    INSERT INTO
        items
            (name, barcode, serial, location, quantity)
        VALUES
            (?1, ?2, ?3, ?4, ?5)
"#;

//pub const 

pub const DELETE_ITEM: &'static str = r#"
    DELETE FROM
        items
    WHERE
        id = ?1
"#;
