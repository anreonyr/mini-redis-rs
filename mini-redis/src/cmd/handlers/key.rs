use bytes::Bytes;
use tokio::time::Instant;

use crate::storage::db::{with_db, Value};
use crate::protocol::resp::RespType;

pub fn handle_del(keys: &[String]) -> RespType {
    with_db(|db| {
        let mut deleted = 0i64;
        for key in keys {
            if db.contains_key(key) {
                db.remove(key);
                crate::storage::db::bump_version();
                deleted += 1;
            }
        }
        RespType::Integer(deleted)
    })
}

pub fn handle_exists(keys: &[String]) -> RespType {
    with_db(|db| {
        let mut count = 0i64;
        for key in keys {
            if let Some(entry) = db.get(key) {
                if entry.expiry.is_some_and(|exp| Instant::now() >= exp) {
                    continue;
                }
                count += 1;
            }
        }
        RespType::Integer(count)
    })
}

pub fn handle_type(key: &str) -> RespType {
    with_db(|db| {
        let typ = match db.get(key) {
            Some(entry) => {
                if entry.expiry.is_some_and(|exp| Instant::now() >= exp) {
                    "none"
                } else {
                    match &entry.value {
                        Value::String(_) => "string",
                        Value::List(_) => "list",
                        Value::Stream(_) => "stream",
                        Value::Hash(_) => "hash",
                        Value::Set(_) => "set",
                        Value::ZSet(_) => "zset",
                    }
                }
            }
            None => "none",
        };
        RespType::SimpleString(typ.to_string())
    })
}

pub fn handle_keys(pattern: &str) -> RespType {
    with_db(|db| {
        let parts: Vec<&str> = pattern.split('*').collect();
        let matching: Vec<RespType> = db
            .iter()
            .filter(|(key, entry)| {
                let key: &str = key;
                if entry.expiry.is_some_and(|exp| Instant::now() >= exp) {
                    return false;
                }
                if parts.len() == 1 {
                    return key == parts[0];
                }
                if !key.starts_with(parts[0]) {
                    return false;
                }
                if !key.ends_with(parts[parts.len() - 1]) {
                    return false;
                }
                let mut pos = parts[0].len();
                for &part in &parts[1..parts.len() - 1] {
                    if let Some(found) = key[pos..].find(part) {
                        pos += found + part.len();
                    } else {
                        return false;
                    }
                }
                true
            })
            .map(|(key, _)| RespType::BulkString(Some(Bytes::from(key.clone()))))
            .collect();
        RespType::Array(Some(matching))
    })
}

pub fn handle_dbsize() -> RespType {
    with_db(|db| {
        let count = db
            .iter()
            .filter(|(_, entry)| !entry.expiry.is_some_and(|exp| Instant::now() >= exp))
            .count() as i64;
        RespType::Integer(count)
    })
}

pub fn handle_rename(key: &str, newkey: &str) -> RespType {
    with_db(|db| match db.remove(key) {
        Some(entry) => {
            db.insert(newkey.to_string(), entry);
            crate::storage::db::bump_version();
            RespType::SimpleString("OK".to_string())
        }
        None => RespType::Error("ERR no such key".to_string()),
    })
}

pub fn handle_renamenx(key: &str, newkey: &str) -> RespType {
    with_db(|db| {
        if db.contains_key(newkey) {
            return RespType::Integer(0);
        }
        match db.remove(key) {
            Some(entry) => {
                db.insert(newkey.to_string(), entry);
                crate::storage::db::bump_version();
                RespType::Integer(1)
            }
            None => RespType::Error("ERR no such key".to_string()),
        }
    })
}

pub fn handle_touch(keys: &[String]) -> RespType {
    with_db(|db| {
        let mut count = 0i64;
        for key in keys {
            if let Some(entry) = db.get_mut(key) {
                if !entry.expiry.is_some_and(|exp| Instant::now() >= exp) {
                    count += 1;
                }
            }
        }
        RespType::Integer(count)
    })
}

pub fn handle_randomkey() -> RespType {
    with_db(|db| {
        let valid: Vec<&String> = db
            .iter()
            .filter(|(_, entry)| !entry.expiry.is_some_and(|exp| Instant::now() >= exp))
            .map(|(k, _)| k)
            .collect();
        if valid.is_empty() {
            RespType::BulkString(None)
        } else {
            let idx = valid.len() / 2; // simple pseudo-random
            RespType::BulkString(Some(Bytes::from(valid[idx].clone())))
        }
    })
}
