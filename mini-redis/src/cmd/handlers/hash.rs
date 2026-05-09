use crate::db::{with_db, Value};
use crate::resp::RespType;

pub fn handle_hset(key: &str, fields: &[(String, String)]) -> RespType {
    with_db(|db| {
        let entry = db.entry(key.to_string()).or_insert_with(|| {
            crate::db::Entry::new(
                Value::Hash(std::collections::HashMap::new()),
                None,
            )
        });
        match &mut entry.value {
            Value::Hash(map) => {
                let mut new_count = 0i64;
                for (f, v) in fields {
                    let f_bytes = f.clone().into_bytes();
                    let v_bytes = v.clone().into_bytes();
                    if map.insert(bytes::Bytes::from(f_bytes), bytes::Bytes::from(v_bytes)).is_none() {
                        new_count += 1;
                    }
                }
                RespType::Integer(new_count)
            }
            _ => wrong_type(),
        }
    })
}

pub fn handle_hget(key: &str, field: &str) -> RespType {
    with_db(|db| match db.get(key) {
        Some(entry) => match &entry.value {
            Value::Hash(map) => {
                let key = bytes::Bytes::from(field.as_bytes().to_vec());
                match map.get(&key) {
                    Some(v) => RespType::BulkString(Some(v.clone())),
                    None => RespType::BulkString(None),
                }
            }
            _ => wrong_type(),
        },
        None => RespType::BulkString(None),
    })
}

pub fn handle_hdel(key: &str, fields: &[String]) -> RespType {
    with_db(|db| match db.get_mut(key) {
        Some(entry) => match &mut entry.value {
            Value::Hash(map) => {
                let mut removed = 0i64;
                for f in fields {
                    let f_bytes = bytes::Bytes::from(f.as_bytes().to_vec());
                    if map.remove(&f_bytes).is_some() {
                        removed += 1;
                    }
                }
                if map.is_empty() {
                    db.remove(key);
                }
                RespType::Integer(removed)
            }
            _ => wrong_type(),
        },
        None => RespType::Integer(0),
    })
}

pub fn handle_hgetall(key: &str) -> RespType {
    let _ = key;
    RespType::Error("ERR not implemented".to_string())
}

pub fn handle_hexists(key: &str, field: &str) -> RespType {
    let _ = (key, field);
    RespType::Error("ERR not implemented".to_string())
}

pub fn handle_hlen(key: &str) -> RespType {
    let _ = key;
    RespType::Error("ERR not implemented".to_string())
}

pub fn handle_hkeys(key: &str) -> RespType {
    let _ = key;
    RespType::Error("ERR not implemented".to_string())
}

pub fn handle_hvals(key: &str) -> RespType {
    let _ = key;
    RespType::Error("ERR not implemented".to_string())
}

fn wrong_type() -> RespType {
    RespType::Error(
        "WRONGTYPE Operation against a key holding the wrong kind of value".to_string(),
    )
}
