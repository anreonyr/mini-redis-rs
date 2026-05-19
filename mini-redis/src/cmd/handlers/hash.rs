use crate::storage::db::{Value, with_db};
use crate::protocol::resp::RespType;

pub fn handle_hset(key: &str, fields: &[(String, String)]) -> RespType {
    with_db(|db| {
        let entry = db.entry(key.to_string()).or_insert_with(|| {
            crate::storage::db::Entry::new(Value::Hash(std::collections::HashMap::new()), None)
        });
        match &mut entry.value {
            Value::Hash(map) => {
                let mut new_count = 0i64;
                for (f, v) in fields {
                    let f_bytes = f.clone().into_bytes();
                    let v_bytes = v.clone().into_bytes();
                    if map
                        .insert(bytes::Bytes::from(f_bytes), bytes::Bytes::from(v_bytes))
                        .is_none()
                    {
                        new_count += 1;
                    }
                }
                entry.version = crate::storage::db::bump_version();
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
                entry.version = crate::storage::db::bump_version();
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
    with_db(|db| match db.get(key) {
        Some(entry) => match &entry.value {
            Value::Hash(map) => {
                let mut items: Vec<RespType> = Vec::with_capacity(map.len() * 2);
                for (f, v) in map {
                    items.push(RespType::BulkString(Some(f.clone())));
                    items.push(RespType::BulkString(Some(v.clone())));
                }
                RespType::Array(Some(items))
            }
            _ => wrong_type(),
        },
        None => RespType::Array(Some(Vec::new())),
    })
}

pub fn handle_hexists(key: &str, field: &str) -> RespType {
    with_db(|db| match db.get(key) {
        Some(entry) => match &entry.value {
            Value::Hash(map) => {
                let exists = map.contains_key(&bytes::Bytes::from(field.as_bytes().to_vec()));
                RespType::Integer(if exists { 1 } else { 0 })
            }
            _ => wrong_type(),
        },
        None => RespType::Integer(0),
    })
}

pub fn handle_hlen(key: &str) -> RespType {
    with_db(|db| match db.get(key) {
        Some(entry) => match &entry.value {
            Value::Hash(map) => RespType::Integer(map.len() as i64),
            _ => wrong_type(),
        },
        None => RespType::Integer(0),
    })
}

pub fn handle_hkeys(key: &str) -> RespType {
    with_db(|db| match db.get(key) {
        Some(entry) => match &entry.value {
            Value::Hash(map) => {
                let fields: Vec<RespType> = map
                    .keys()
                    .map(|k| RespType::BulkString(Some(k.clone())))
                    .collect();
                RespType::Array(Some(fields))
            }
            _ => wrong_type(),
        },
        None => RespType::Array(Some(Vec::new())),
    })
}

pub fn handle_hvals(key: &str) -> RespType {
    with_db(|db| match db.get(key) {
        Some(entry) => match &entry.value {
            Value::Hash(map) => {
                let vals: Vec<RespType> = map
                    .values()
                    .map(|v| RespType::BulkString(Some(v.clone())))
                    .collect();
                RespType::Array(Some(vals))
            }
            _ => wrong_type(),
        },
        None => RespType::Array(Some(Vec::new())),
    })
}

fn wrong_type() -> RespType {
    RespType::Error("WRONGTYPE Operation against a key holding the wrong kind of value".to_string())
}

pub fn handle_hincrby(key: &str, field: &str, incr: i64) -> RespType {
    with_db(|db| {
        let entry = db.entry(key.to_string()).or_insert_with(|| {
            crate::storage::db::Entry::new(Value::Hash(std::collections::HashMap::new()), None)
        });
        match &mut entry.value {
            Value::Hash(hash) => {
                let mb = bytes::Bytes::from(field.as_bytes().to_vec());
                let current: i64 = hash
                    .get(&mb)
                    .and_then(|v| std::str::from_utf8(v).ok())
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                let new_val = current.wrapping_add(incr);
                hash.insert(mb, bytes::Bytes::from(new_val.to_string()));
                entry.version = crate::storage::db::bump_version();
                RespType::Integer(new_val)
            }
            _ => wrong_type(),
        }
    })
}

pub fn handle_hincrbyfloat(key: &str, field: &str, incr: f64) -> RespType {
    with_db(|db| {
        let entry = db.entry(key.to_string()).or_insert_with(|| {
            crate::storage::db::Entry::new(Value::Hash(std::collections::HashMap::new()), None)
        });
        match &mut entry.value {
            Value::Hash(hash) => {
                let mb = bytes::Bytes::from(field.as_bytes().to_vec());
                let current: f64 = hash
                    .get(&mb)
                    .and_then(|v| std::str::from_utf8(v).ok())
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0);
                let new_val = current + incr;
                hash.insert(mb, bytes::Bytes::from(format!("{}", new_val).into_bytes()));
                entry.version = crate::storage::db::bump_version();
                RespType::BulkString(Some(bytes::Bytes::from(format!("{}", new_val).into_bytes())))
            }
            _ => wrong_type(),
        }
    })
}

pub fn handle_hrandfield(key: &str, count: Option<i64>, withvalues: bool) -> RespType {
    with_db(|db| match db.get(key) {
        Some(entry) => match &entry.value {
            Value::Hash(map) => {
                if map.is_empty() {
                    return match count {
                        Some(_) => RespType::Array(Some(vec![])),
                        None => RespType::BulkString(None),
                    };
                }
                let fields: Vec<(&bytes::Bytes, &bytes::Bytes)> = map.iter().collect();
                match count {
                    None => {
                        // Return a single random field
                        let (field, _) = fields[0];
                        RespType::BulkString(Some(field.clone()))
                    }
                    Some(c) if c >= 0 => {
                        // Return up to c unique fields (no duplicates)
                        let n = (c as usize).min(fields.len());
                        let mut results = Vec::with_capacity(if withvalues { n * 2 } else { n });
                        for i in 0..n {
                            let (field, value) = fields[i];
                            results.push(RespType::BulkString(Some(field.clone())));
                            if withvalues {
                                results.push(RespType::BulkString(Some(value.clone())));
                            }
                        }
                        RespType::Array(Some(results))
                    }
                    Some(c) => {
                        // Return exactly |c| fields, allowing duplicates (cycle through all fields)
                        let n = (-c) as usize;
                        let mut results = Vec::with_capacity(if withvalues { n * 2 } else { n });
                        for i in 0..n {
                            let (field, value) = fields[i % fields.len()];
                            results.push(RespType::BulkString(Some(field.clone())));
                            if withvalues {
                                results.push(RespType::BulkString(Some(value.clone())));
                            }
                        }
                        RespType::Array(Some(results))
                    }
                }
            }
            _ => wrong_type(),
        },
        None => match count {
            Some(_) => RespType::Array(Some(vec![])),
            None => RespType::BulkString(None),
        },
    })
}

pub fn handle_hstrlen(key: &str, field: &str) -> RespType {
    with_db(|db| match db.get(key) {
        Some(entry) => match &entry.value {
            Value::Hash(map) => {
                let k = bytes::Bytes::from(field.as_bytes().to_vec());
                match map.get(&k) {
                    Some(v) => RespType::Integer(v.len() as i64),
                    None => RespType::Integer(0),
                }
            }
            _ => wrong_type(),
        },
        None => RespType::Integer(0),
    })
}

pub fn handle_hsetnx(key: &str, field: &str, value: &str) -> RespType {
    with_db(|db| {
        let entry = db.entry(key.to_string()).or_insert_with(|| {
            crate::storage::db::Entry::new(Value::Hash(std::collections::HashMap::new()), None)
        });
        match &mut entry.value {
            Value::Hash(hash) => {
                let mb = bytes::Bytes::from(field.as_bytes().to_vec());
                if hash.contains_key(&mb) {
                    RespType::Integer(0)
                } else {
                    hash.insert(mb, bytes::Bytes::from(value.as_bytes().to_vec()));
                    entry.version = crate::storage::db::bump_version();
                    RespType::Integer(1)
                }
            }
            _ => wrong_type(),
        }
    })
}
