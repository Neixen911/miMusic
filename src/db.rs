use std::sync::{OnceLock, Mutex};
use rusqlite::{Connection};

pub struct Database {
    db: Connection,
}

impl Database {
    fn new() -> Self {
        Database { db: Connection::open("./miMusic.db").expect("Can't connect to the database !") }
    }

    pub fn get_instance() -> &'static Mutex<Connection> {
        static INSTANCE: OnceLock<Mutex<Connection>> = OnceLock::new();
        INSTANCE.get_or_init(|| Mutex::new(Database::new().db))
    }
}
