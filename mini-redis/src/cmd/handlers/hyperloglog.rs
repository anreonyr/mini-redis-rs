use hyperloglog::HyperLogLog;

use crate::db::{with_db, Entry, Value};
use crate::resp::RespType;

/// Error rate that yields p=14 (16384 registers, ~0.81% standard error),
/// matching Redis's HyperLogLog precision.
const HLL_ERROR_RATE: f64 = 0.001;

fn wrong_type() -> RespType {
    RespType::Error(
        "WRONGTYPE Operation against a key holding the wrong kind of value".to_string(),
    )
}

/// Deserialize a `HyperLogLog` from its stored bytes (bincode).
fn hll_from_bytes(bytes: &[u8]) -> Option<HyperLogLog> {
    bincode::deserialize::<HyperLogLog>(bytes).ok()
}

/// Serialize a `HyperLogLog` into bytes for storage.
fn hll_to_bytes(hll: &HyperLogLog) -> Vec<u8> {
    bincode::serialize(hll).expect("HyperLogLog serialization failed")
}

/// Helper to get the cardinality of a stored HLL.
fn hll_count(bytes: &[u8]) -> Option<u64> {
    hll_from_bytes(bytes).map(|h| h.len() as u64)
}

pub fn handle_pfadd(key: &str, elements: &[String]) -> RespType {
    with_db(|db| {
        // Type check: if key exists, it must be a HyperLogLog
        if let Some(entry) = db.get(key) {
            match &entry.value {
                Value::HyperLogLog(_) => {}
                _ => return wrong_type(),
            }
        }

        let old_count = db.get(key).and_then(|e| match &e.value {
            Value::HyperLogLog(bytes) => hll_count(bytes),
            _ => None,
        });

        let entry = db.entry(key.to_string()).or_insert_with(|| {
            Entry::new(
                Value::HyperLogLog(hll_to_bytes(&HyperLogLog::new(HLL_ERROR_RATE))),
                None,
            )
        });

        match &mut entry.value {
            Value::HyperLogLog(bytes) => {
                let mut hll = hll_from_bytes(bytes)
                    .unwrap_or_else(|| HyperLogLog::new(HLL_ERROR_RATE));
                for elem in elements {
                    hll.insert(elem);
                }
                *bytes = hll_to_bytes(&hll);
                entry.version = crate::db::bump_version();
                let new_count = hll.len() as u64;
                // Return 1 if cardinality changed, 0 otherwise
                if old_count.is_none() || old_count.unwrap() != new_count {
                    RespType::Integer(1)
                } else {
                    RespType::Integer(0)
                }
            }
            _ => wrong_type(),
        }
    })
}

pub fn handle_pfcount(keys: &[String]) -> RespType {
    with_db(|db| {
        // Type check all keys
        for key in keys {
            if let Some(entry) = db.get(key) {
                match &entry.value {
                    Value::HyperLogLog(_) => {}
                    _ => return wrong_type(),
                }
            }
        }

        let mut hll: Option<HyperLogLog> = None;
        for key in keys {
            if let Some(entry) = db.get(key) {
                if let Value::HyperLogLog(bytes) = &entry.value {
                    if let Some(other) = hll_from_bytes(bytes) {
                        match &mut hll {
                            Some(h) => h.merge(&other),
                            None => hll = Some(other),
                        }
                    }
                }
            }
        }
        match hll {
            Some(h) => RespType::Integer(h.len() as i64),
            None => RespType::Integer(0),
        }
    })
}

pub fn handle_pfmerge(dest: &str, sources: &[String]) -> RespType {
    with_db(|db| {
        // Type check all sources
        for key in sources {
            if let Some(entry) = db.get(key) {
                match &entry.value {
                    Value::HyperLogLog(_) => {}
                    _ => return wrong_type(),
                }
            }
        }
        // Type check dest if it exists
        if let Some(entry) = db.get(dest) {
            match &entry.value {
                Value::HyperLogLog(_) => {}
                _ => return wrong_type(),
            }
        }

        // Merge all source HLLs
        let mut merged: Option<HyperLogLog> = None;
        for key in sources {
            if let Some(entry) = db.get(key) {
                if let Value::HyperLogLog(bytes) = &entry.value {
                    if let Some(other) = hll_from_bytes(bytes) {
                        match &mut merged {
                            Some(h) => h.merge(&other),
                            None => merged = Some(other),
                        }
                    }
                }
            }
        }

        // Get or create dest entry
        let entry = db.entry(dest.to_string()).or_insert_with(|| {
            Entry::new(
                Value::HyperLogLog(
                    merged
                        .as_ref()
                        .map(|h| hll_to_bytes(h))
                        .unwrap_or_else(|| hll_to_bytes(&HyperLogLog::new(HLL_ERROR_RATE))),
                ),
                None,
            )
        });

        match &mut entry.value {
            Value::HyperLogLog(bytes) => {
                let mut hll = hll_from_bytes(bytes)
                    .unwrap_or_else(|| HyperLogLog::new(HLL_ERROR_RATE));
                if let Some(ref source) = merged {
                    hll.merge(source);
                }
                *bytes = hll_to_bytes(&hll);
                entry.version = crate::db::bump_version();
                RespType::SimpleString("OK".to_string())
            }
            _ => wrong_type(),
        }
    })
}
