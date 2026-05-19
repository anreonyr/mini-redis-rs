use bytes::Bytes;
use tokio::time::Instant;

use crate::db::{Entry, Value, with_db};
use crate::resp;
use crate::resp::RespType;
use std::time::Duration;

pub fn handle_set(key: &str, value: &str, expiry: Option<Duration>) -> RespType {
    with_db(|db| {
        db.insert(
            key.to_string(),
            Entry::new(
                Value::String(Bytes::from(value.to_string())),
                expiry.map(|d| Instant::now() + d),
            ),
        );
    });
    RespType::SimpleString("OK".to_string())
}

pub fn handle_get(key: &str) -> RespType {
    with_db(|db| match db.get(key) {
        Some(entry) => {
            if entry.expiry.is_some_and(|exp| Instant::now() >= exp) {
                db.remove(key);
                RespType::BulkString(None)
            } else {
                match entry.value.clone() {
                    Value::String(v) => RespType::BulkString(Some(v)),
                    _ => wrong_type(),
                }
            }
        }
        None => RespType::BulkString(None),
    })
}

pub fn handle_incr(key: &str) -> RespType {
    incrby(key, 1)
}

pub fn handle_decr(key: &str) -> RespType {
    incrby(key, -1)
}

pub fn handle_incrby(key: &str, delta: i64) -> RespType {
    incrby(key, delta)
}

pub fn handle_decrby(key: &str, delta: i64) -> RespType {
    incrby(key, -delta)
}

fn incrby(key: &str, delta: i64) -> RespType {
    with_db(|db| {
        let entry = db
            .entry(key.to_string())
            .and_modify(|e| {
                if e.expiry.is_some_and(|exp| Instant::now() >= exp) {
                    e.expiry = None;
                    e.value = Value::String(Bytes::from("0"));
                }
            })
            .or_insert_with(|| Entry::new(Value::String(Bytes::from("0")), None));

        match &entry.value {
            Value::String(v) => {
                let current: i64 = match std::str::from_utf8(v).ok().and_then(|s| s.parse().ok()) {
                    Some(n) => n,
                    None => {
                        return RespType::Error(
                            "ERR value is not an integer or out of range".to_string(),
                        );
                    }
                };
                let new_val = current.wrapping_add(delta);
                entry.value = Value::String(Bytes::from(new_val.to_string()));
                RespType::Integer(new_val)
            }
            _ => wrong_type(),
        }
    })
}

pub fn handle_append(key: &str, value: &str) -> RespType {
    with_db(|db| {
        let entry = db.entry(key.to_string()).or_insert_with(|| {
            Entry::new(Value::String(Bytes::new()), None)
        });
        match &mut entry.value {
            Value::String(v) => {
                let mut data = v.to_vec();
                data.extend_from_slice(value.as_bytes());
                *v = Bytes::from(data);
                RespType::Integer(v.len() as i64)
            }
            _ => wrong_type(),
        }
    })
}

pub fn handle_strlen(key: &str) -> RespType {
    with_db(|db| match db.get(key) {
        Some(entry) => {
            if entry.expiry.is_some_and(|exp| Instant::now() >= exp) {
                db.remove(key);
                return RespType::Integer(0);
            }
            match &entry.value {
                Value::String(v) => RespType::Integer(v.len() as i64),
                _ => wrong_type(),
            }
        }
        None => RespType::Integer(0),
    })
}

pub fn handle_mget(keys: &[String]) -> RespType {
    with_db(|db| {
        let results: Vec<RespType> = keys
            .iter()
            .map(|key| match db.get(key) {
                Some(entry) => {
                    if entry.expiry.is_some_and(|exp| Instant::now() >= exp) {
                        RespType::BulkString(None)
                    } else {
                        match &entry.value {
                            Value::String(v) => RespType::BulkString(Some(v.clone())),
                            _ => RespType::BulkString(None),
                        }
                    }
                }
                None => RespType::BulkString(None),
            })
            .collect();
        RespType::Array(Some(results))
    })
}

pub fn handle_mset(pairs: &[(String, String)]) -> RespType {
    with_db(|db| {
        for (key, value) in pairs {
            db.insert(
                key.clone(),
                Entry::new(Value::String(Bytes::from(value.clone())), None),
            );
        }
    });
    RespType::SimpleString("OK".to_string())
}

pub fn handle_getset(key: &str, value: &str) -> RespType {
    with_db(|db| {
        let old = db.get(key).and_then(|entry| {
            if entry.expiry.is_some_and(|exp| Instant::now() >= exp) {
                None
            } else {
                match &entry.value {
                    Value::String(v) => Some(v.clone()),
                    _ => None,
                }
            }
        });
        if let Some(old_val) = old {
            db.insert(
                key.to_string(),
                Entry::new(Value::String(Bytes::from(value.to_string())), None),
            );
            RespType::BulkString(Some(old_val))
        } else if db.contains_key(key) {
            // wrong type
            wrong_type()
        } else {
            RespType::BulkString(None)
        }
    })
}

pub fn handle_getrange(key: &str, start: i64, end: i64) -> RespType {
    with_db(|db| match db.get(key) {
        Some(entry) => {
            if entry.expiry.is_some_and(|exp| Instant::now() >= exp) {
                db.remove(key);
                return RespType::BulkString(Some(Bytes::new()));
            }
            match &entry.value {
                Value::String(v) => {
                    let len = v.len() as i64;
                    if len == 0 {
                        return RespType::BulkString(Some(Bytes::new()));
                    }
                    let s = if start < 0 { (len + start).max(0) } else { start.min(len - 1) };
                    let e = if end < 0 { (len + end).max(0) } else { end.min(len - 1) };
                    if s > e || s >= len {
                        RespType::BulkString(Some(Bytes::new()))
                    } else {
                        let slice = v.slice(s as usize..(e + 1) as usize);
                        RespType::BulkString(Some(slice))
                    }
                }
                _ => wrong_type(),
            }
        }
        None => RespType::BulkString(Some(Bytes::new())),
    })
}

pub fn handle_setrange(key: &str, offset: u64, value: &str) -> RespType {
    with_db(|db| {
        let entry = db.entry(key.to_string()).or_insert_with(|| {
            Entry::new(Value::String(Bytes::new()), None)
        });
        match &mut entry.value {
            Value::String(v) => {
                let off = offset as usize;
                let val_bytes = value.as_bytes();
                let needed = off + val_bytes.len();
                if needed > v.len() {
                    let mut new_data = v.to_vec();
                    new_data.resize(needed, 0);
                    new_data[off..off + val_bytes.len()].copy_from_slice(val_bytes);
                    *v = Bytes::from(new_data);
                } else {
                    let mut new_data = v.to_vec();
                    new_data[off..off + val_bytes.len()].copy_from_slice(val_bytes);
                    *v = Bytes::from(new_data);
                }
                RespType::Integer(v.len() as i64)
            }
            _ => wrong_type(),
        }
    })
}

pub fn handle_msetnx(pairs: &[(String, String)]) -> RespType {
    with_db(|db| {
        for (key, _) in pairs {
            if db.contains_key(key) {
                return RespType::Integer(0);
            }
        }
        for (key, value) in pairs {
            db.insert(
                key.clone(),
                Entry::new(Value::String(Bytes::from(value.clone())), None),
            );
        }
        RespType::Integer(1)
    })
}

fn wrong_type() -> RespType {
    resp::RespType::Error(
        "WRONGTYPE Operation against a key holding the wrong kind of value".to_string(),
    )
}
