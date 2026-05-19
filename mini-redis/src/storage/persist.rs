use std::collections::HashMap;
use std::fs;
use std::time::Duration;

use tokio::time::Instant;

use crate::storage::db::Entry;

/// Save the entire database to a file at `path`.
/// Expired keys are skipped during serialization.
/// Runs blocking IO on a dedicated thread via `spawn_blocking`.
/// Times out after 30 seconds to avoid hanging the server on large datasets.
pub async fn save(path: &str) -> Result<(), String> {
    let data = crate::storage::db::with_db(|db| {
        let mut map: HashMap<String, Entry> = HashMap::new();
        let now = Instant::now();
        for (key, entry) in db.iter() {
            if entry.expiry.is_some_and(|exp| now >= exp) {
                continue;
            }
            map.insert(key.clone(), entry.clone());
        }
        map
    });

    let path = path.to_string();
    let result = tokio::time::timeout(Duration::from_secs(30), async {
        tokio::task::spawn_blocking(move || {
            let bytes =
                bincode::serialize(&data).map_err(|e| format!("serialize error: {}", e))?;

            // Atomic write: write to temp file, then rename
            let tmp = format!("{}.tmp", path);
            fs::write(&tmp, &bytes).map_err(|e| format!("write error: {}", e))?;
            fs::rename(&tmp, &path).map_err(|e| format!("rename error: {}", e))?;
            Ok::<_, String>(())
        })
        .await
        .map_err(|e| format!("task join error: {}", e))?
    })
    .await;

    match result {
        Ok(inner) => inner,
        Err(_) => Err("save timed out after 30s".to_string()),
    }
}

/// Load the database from a file at `path`.
/// Replaces all current in-memory data. Returns the number of keys loaded.
/// Runs blocking IO synchronously (called once at startup before the event loop).
pub fn load(path: &str) -> Result<usize, String> {
    let bytes = fs::read(path).map_err(|e| format!("read error: {}", e))?;
    let data: HashMap<String, Entry> =
        bincode::deserialize(&bytes).map_err(|e| format!("deserialize error: {}", e))?;

    let count = data.len();
    crate::storage::db::with_db(|db| {
        db.clear();
        db.extend(data);
    });
    Ok(count)
}

/// Check whether a persistence file exists at `path`.
pub fn file_exists(path: &str) -> bool {
    std::path::Path::new(path).exists()
}
