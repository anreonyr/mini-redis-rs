use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use tokio::time::Instant;

use crate::db::{Entry, Value};

/// Persistence-specific data format (handles non-serializable `Instant` in Entry.expiry).
#[derive(Serialize, Deserialize)]
pub struct PersistEntry {
    value: Value,
}

impl From<&Entry> for PersistEntry {
    fn from(entry: &Entry) -> Self {
        PersistEntry {
            value: entry.value.clone(),
        }
    }
}

impl From<PersistEntry> for Entry {
    fn from(pe: PersistEntry) -> Self {
        Entry {
            value: pe.value,
            expiry: None,
        }
    }
}

/// Save the entire database to a file at `path`.
/// Expired keys are skipped during serialization.
pub fn save(path: &str) -> Result<(), String> {
    let data = crate::db::with_db(|db| {
        let mut map: HashMap<String, PersistEntry> = HashMap::new();
        let now = Instant::now();
        for (key, entry) in db.iter() {
            if entry.expiry.is_some_and(|exp| now >= exp) {
                continue;
            }
            map.insert(key.clone(), PersistEntry::from(entry));
        }
        map
    });

    let bytes = bincode::serialize(&data).map_err(|e| format!("serialize error: {}", e))?;
    fs::write(path, &bytes).map_err(|e| format!("write error: {}", e))?;
    Ok(())
}

/// Load the database from a file at `path`.
/// Returns the number of keys loaded.
pub fn load(path: &str) -> Result<usize, String> {
    let bytes = fs::read(path).map_err(|e| format!("read error: {}", e))?;
    let data: HashMap<String, PersistEntry> =
        bincode::deserialize(&bytes).map_err(|e| format!("deserialize error: {}", e))?;

    let count = data.len();
    crate::db::with_db(|db| {
        db.clear();
        for (key, pe) in data {
            db.insert(key, Entry::from(pe));
        }
    });
    Ok(count)
}

/// Check whether a persistence file exists at `path`.
pub fn file_exists(path: &str) -> bool {
    Path::new(path).exists()
}
