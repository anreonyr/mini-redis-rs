use crate::storage::db::{with_db, Value};
use crate::protocol::resp::RespType;

pub fn handle_sadd(key: &str, members: &[String]) -> RespType {
    with_db(|db| {
        let entry = db.entry(key.to_string()).or_insert_with(|| {
            crate::storage::db::Entry::new(Value::Set(std::collections::HashSet::new()), None)
        });
        match &mut entry.value {
            Value::Set(set) => {
                let mut new_count = 0i64;
                for m in members {
                    if set.insert(bytes::Bytes::copy_from_slice(m.as_bytes())) {
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

pub fn handle_smembers(key: &str) -> RespType {
    with_db(|db| match db.get(key) {
        Some(entry) => match &entry.value {
            Value::Set(set) => {
                let members: Vec<RespType> = set
                    .iter()
                    .map(|m| RespType::BulkString(Some(m.clone())))
                    .collect();
                RespType::Array(Some(members))
            }
            _ => wrong_type(),
        },
        None => RespType::Array(Some(Vec::new())),
    })
}

pub fn handle_sismember(key: &str, member: &str) -> RespType {
    with_db(|db| match db.get(key) {
        Some(entry) => match &entry.value {
            Value::Set(set) => {
                let exists = set.contains(&bytes::Bytes::copy_from_slice(member.as_bytes()));
                RespType::Integer(if exists { 1 } else { 0 })
            }
            _ => wrong_type(),
        },
        None => RespType::Integer(0),
    })
}

pub fn handle_srem(key: &str, members: &[String]) -> RespType {
    with_db(|db| match db.get_mut(key) {
        Some(entry) => match &mut entry.value {
            Value::Set(set) => {
                let mut removed = 0i64;
                for m in members {
                    if set.remove(&bytes::Bytes::copy_from_slice(m.as_bytes())) {
                        removed += 1;
                    }
                }
                entry.version = crate::storage::db::bump_version();
                if set.is_empty() {
                    db.remove(key);
                }
                RespType::Integer(removed)
            }
            _ => wrong_type(),
        },
        None => RespType::Integer(0),
    })
}

pub fn handle_scard(key: &str) -> RespType {
    with_db(|db| match db.get(key) {
        Some(entry) => match &entry.value {
            Value::Set(set) => RespType::Integer(set.len() as i64),
            _ => wrong_type(),
        },
        None => RespType::Integer(0),
    })
}

pub fn handle_spop(key: &str, count: Option<usize>) -> RespType {
    let n = count.unwrap_or(1);
    with_db(|db| match db.get_mut(key) {
        Some(entry) => match &mut entry.value {
            Value::Set(set) => {
                if set.is_empty() || n == 0 {
                    return match count {
                        None => RespType::BulkString(None),
                        Some(_) => RespType::Array(Some(vec![])),
                    };
                }
                let mut popped: Vec<bytes::Bytes> = Vec::new();
                for _ in 0..n {
                    let elem = set.iter().next().cloned();
                    match elem {
                        Some(e) => {
                            set.remove(&e);
                            popped.push(e);
                        }
                        None => break,
                    }
                }
                entry.version = crate::storage::db::bump_version();
                if set.is_empty() {
                    db.remove(key);
                }
                match count {
                    None => RespType::BulkString(Some(popped.into_iter().next().unwrap())),
                    Some(_) if popped.is_empty() => RespType::Array(None),
                    Some(_) => {
                        let items: Vec<RespType> = popped
                            .into_iter()
                            .map(|b| RespType::BulkString(Some(b)))
                            .collect();
                        RespType::Array(Some(items))
                    }
                }
            }
            _ => wrong_type(),
        },
        None => match count {
            None => RespType::BulkString(None),
            Some(_) => RespType::Array(None),
        },
    })
}

pub fn handle_srandmember(key: &str, count: Option<i64>) -> RespType {
    with_db(|db| match db.get(key) {
        Some(entry) => match &entry.value {
            Value::Set(set) => {
                if set.is_empty() {
                    return match count {
                        Some(_) => RespType::Array(Some(vec![])),
                        None => RespType::BulkString(None),
                    };
                }
                let members: Vec<&bytes::Bytes> = set.iter().collect();
                match count {
                    None => {
                        let elem = set.iter().next().unwrap();
                        RespType::BulkString(Some(elem.clone()))
                    }
                    Some(c) if c >= 0 => {
                        let n = (c as usize).min(members.len());
                        let items: Vec<RespType> = members[..n]
                            .iter()
                            .map(|b| RespType::BulkString(Some((*b).clone())))
                            .collect();
                        RespType::Array(Some(items))
                    }
                    Some(c) => {
                        let n = (-c) as usize;
                        let items: Vec<RespType> = (0..n)
                            .map(|_| {
                                let elem = set.iter().next().unwrap();
                                RespType::BulkString(Some(elem.clone()))
                            })
                            .collect();
                        RespType::Array(Some(items))
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

pub fn handle_sunion(keys: &[String]) -> RespType {
    with_db(|db| {
        let mut result = std::collections::HashSet::new();
        for key in keys {
            if let Some(entry) = db.get(key) {
                if let Value::Set(set) = &entry.value {
                    result.extend(set.iter().cloned());
                } else {
                    return wrong_type();
                }
            }
        }
        let items: Vec<RespType> = result
            .into_iter()
            .map(|b| RespType::BulkString(Some(b)))
            .collect();
        RespType::Array(Some(items))
    })
}

pub fn handle_sinter(keys: &[String]) -> RespType {
    with_db(|db| {
        let first = keys.first();
        if first.is_none() {
            return RespType::Array(Some(vec![]));
        }
        let first = first.unwrap();
        let base = match db.get(first) {
            Some(entry) => match &entry.value {
                Value::Set(set) => set.clone(),
                _ => return wrong_type(),
            },
            None => return RespType::Array(Some(vec![])),
        };
        let mut result = base;
        for key in &keys[1..] {
            match db.get(key) {
                Some(entry) => match &entry.value {
                    Value::Set(set) => {
                        result = result.intersection(set).cloned().collect();
                    }
                    _ => return wrong_type(),
                },
                None => return RespType::Array(Some(vec![])),
            }
        }
        let items: Vec<RespType> = result
            .into_iter()
            .map(|b| RespType::BulkString(Some(b)))
            .collect();
        RespType::Array(Some(items))
    })
}

pub fn handle_sdiff(keys: &[String]) -> RespType {
    with_db(|db| {
        let first = keys.first();
        if first.is_none() {
            return RespType::Array(Some(vec![]));
        }
        let first = first.unwrap();
        let mut result = match db.get(first) {
            Some(entry) => match &entry.value {
                Value::Set(set) => set.clone(),
                _ => return wrong_type(),
            },
            None => return RespType::Array(Some(vec![])),
        };
        for key in &keys[1..] {
            match db.get(key) {
                Some(entry) => match &entry.value {
                    Value::Set(set) => {
                        for elem in set.iter() {
                            result.remove(elem);
                        }
                    }
                    _ => return wrong_type(),
                },
                None => {}
            }
        }
        let items: Vec<RespType> = result
            .into_iter()
            .map(|b| RespType::BulkString(Some(b)))
            .collect();
        RespType::Array(Some(items))
    })
}

pub fn handle_smove(source: &str, destination: &str, member: &str) -> RespType {
    with_db(|db| {
        let mb = bytes::Bytes::copy_from_slice(member.as_bytes());
        let mut removed = false;
        let mut source_empty = false;
        match db.get_mut(source) {
            Some(entry) => match &mut entry.value {
                Value::Set(set) => {
                    if set.remove(&mb) {
                        removed = true;
                        source_empty = set.is_empty();
                        entry.version = crate::storage::db::bump_version();
                    }
                }
                _ => return wrong_type(),
            },
            None => {}
        }
        if removed && source_empty {
            db.remove(source);
            crate::storage::db::bump_version();
        }
        if !removed {
            return RespType::Integer(0);
        }
        let dest_entry = db.entry(destination.to_string()).or_insert_with(|| {
            crate::storage::db::Entry::new(Value::Set(std::collections::HashSet::new()), None)
        });
        match &mut dest_entry.value {
            Value::Set(set) => {
                set.insert(mb);
                dest_entry.version = crate::storage::db::bump_version();
                RespType::Integer(1)
            }
            _ => wrong_type(),
        }
    })
}

fn wrong_type() -> RespType {
    RespType::Error(
        "WRONGTYPE Operation against a key holding the wrong kind of value".to_string(),
    )
}
