// DB data
use std::process;
use rusqlite::{Connection, params};
use crate::db::sql::{DB_NAME, CREATE_SQL, SELECT_ITEMS, INSERT_ITEM, DELETE_ITEM};

#[derive(Clone)]
pub struct Item {
    pub id: u32,
    pub name: String,
    pub barcode: Option<String>,
    pub serial: Option<String>
}

impl From<(u32, InsertItem)> for Item {
    fn from((id, insert_item): (u32, InsertItem)) -> Self {
        Self {
            id: id,
            name: insert_item.name,
            barcode: insert_item.barcode,
            serial: insert_item.serial
        }
    }
}


pub struct InsertItem {
    pub name: String,
    pub barcode: Option<String>,
    pub serial: Option<String>
}

impl From<(String, Option<String>, Option<String>)> for InsertItem {
    fn from((name, barcode, serial): (String, Option<String>, Option<String>)) -> Self {
        Self {
            name: name,
            barcode: barcode,
            serial: serial
        }
    }
}


pub fn connect_db() -> Connection {
    let connenction = Connection::open(DB_NAME);
    match connenction {
        Ok(conn_ok) => {
            let _ = conn_ok.execute(
                CREATE_SQL,
                []
            );
            conn_ok
        },
        Err(e) => {
            eprintln!("Error while creating connection to '{}': {}", DB_NAME, e);
            process::exit(1)
        }
    }
}


pub fn get_items(connection: &Connection) -> Vec<Item> {
    let mut stmt = connection.prepare(SELECT_ITEMS).expect("prepare failed");
    let item_iter = stmt
        .query_map([], |row| {
            Ok(Item {
                id: row.get::<_, i64>(0)? as u32,
                name: row.get(1)?,
                barcode: row.get(2)?,
                serial: row.get(3)?,
            })
        }).expect("query_map failed");
    let mut v = Vec::new();
    for it in item_iter {
        if let Ok(it) = it {
            v.push(it);
        }
    }
    v
}


pub fn try_insert_item(insert_item: &InsertItem, connection: &Connection) {
    connection.execute(
        INSERT_ITEM,
        params![
            insert_item.name,
            insert_item.barcode,
            insert_item.serial
        ]
    ).expect("insert failed");
}


pub enum ItemAdd {
    Ok(Item),
    Err(String)
}


pub fn trim_or_none_value(value: &String) -> Option<String> {
    let v = value.trim();
    if v.is_empty() {
        return None
    }
    Some(v.to_string())
}


pub fn try_create_item(name: &String, barcode: &String, serial: &String, connection: &Connection) -> ItemAdd {
    let name_insert = name.trim().to_string();
    if name_insert.is_empty() {
        return ItemAdd::Err("Name is required".to_string())
    };
    let barcode_insert = trim_or_none_value(barcode);
    let serial_insert = trim_or_none_value(serial);
    let insert_item: InsertItem = (name_insert, barcode_insert, serial_insert).into();
    try_insert_item(&insert_item, &connection);
    let id = connection.last_insert_rowid() as u32;
    ItemAdd::Ok(
        (id, insert_item).into()
    )
}


pub fn try_update_item(item: &Item, connection: &Connection) -> Item {
    
}


pub fn try_delete_item(id: &u32, connection: &Connection) {
    connection.execute(
        DELETE_ITEM,
        params![id]
    ).expect("delete failed");
}
