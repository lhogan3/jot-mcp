use rusqlite::{Connection, Result as SqliteResult};
use std::time::{SystemTime, UNIX_EPOCH};

/// Core Data stores timestamps as seconds since 2001-01-01, not the Unix epoch.
pub const APPLE_EPOCH_OFFSET: u64 = 978_307_200;

/// `JOT_DB_PATH` overrides this (e.g. when running in Docker, where the file
/// is bind-mounted at a container-local path rather than under $HOME).
fn db_path() -> String {
    std::env::var("JOT_DB_PATH").unwrap_or_else(|_| {
        let home = std::env::var("HOME").expect("HOME is not set");
        format!("{home}/Library/Group Containers/group.hirocloud.jotApp/CoreDataStores/Private/private.sqlite")
    })
}

pub fn get_connection() -> SqliteResult<Connection> {
    Connection::open(db_path())
}

pub fn current_apple_time() -> f64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    (now as f64) - (APPLE_EPOCH_OFFSET as f64)
}
