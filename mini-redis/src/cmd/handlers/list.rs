use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use tokio::sync::Notify;

use crate::waiters;
use crate::db::{Entry, Value, with_db};
use crate::resp;
use crate::resp::RespType;

pub fn handle_rpush(key: &str, values: &[String]) -> RespType {
    let values: VecDeque<Bytes> = values.iter().map(|v| Bytes::from(v.clone())).collect();
    let result = with_db(|db| match db.get_mut(key) {
        Some(entry) => {
            if let Value::List(ref mut list) = entry.value {
                list.extend(values);
                entry.version = crate::db::bump_version();
                RespType::Integer(list.len() as i64)
            } else {
                wrong_type()
            }
        }
        None => {
            let len = values.len();
            db.insert(key.to_string(), Entry::new(Value::List(values), None));
            RespType::Integer(len as i64)
        }
    });
    waiters::notify_waiters(key);
    result
}

pub fn handle_lpush(key: &str, values: &[String]) -> RespType {
    let values: VecDeque<Bytes> = values.iter().map(|v| Bytes::from(v.clone())).collect();
    let result = with_db(|db| match db.get_mut(key) {
        Some(entry) => {
            if let Value::List(ref mut list) = entry.value {
                for v in values {
                    list.push_front(v);
                }
                entry.version = crate::db::bump_version();
                RespType::Integer(list.len() as i64)
            } else {
                wrong_type()
            }
        }
        None => {
            let len = values.len();
            let mut list = VecDeque::new();
            for v in values {
                list.push_front(v);
            }
            db.insert(key.to_string(), Entry::new(Value::List(list), None));
            RespType::Integer(len as i64)
        }
    });
    waiters::notify_waiters(key);
    result
}

pub fn handle_lrange(key: &str, start: i64, stop: i64) -> RespType {
    with_db(|db| match db.get(key) {
        Some(entry) => match entry.value.clone() {
            Value::List(list) => {
                let len = list.len() as i64;
                if len == 0 {
                    return RespType::Array(Some(vec![]));
                }

                let mut l = if start < 0 { len + start } else { start };
                let mut r = if stop < 0 { len + stop } else { stop };

                if l < 0 {
                    l = 0;
                }
                if r >= len {
                    r = len - 1;
                }

                if l > r {
                    RespType::Array(Some(vec![]))
                } else {
                    let items: Vec<RespType> = list
                        .range(l as usize..=r as usize)
                        .map(|v| RespType::BulkString(Some(v.clone())))
                        .collect();
                    RespType::Array(Some(items))
                }
            }
            _ => wrong_type(),
        },
        None => RespType::Array(Some(vec![])),
    })
}

pub fn handle_llen(key: &str) -> RespType {
    with_db(|db| match db.get(key) {
        Some(v) => match &v.value {
            Value::List(u) => RespType::Integer(u.len() as i64),
            _ => wrong_type(),
        },
        None => RespType::Integer(0),
    })
}

pub fn handle_lpop(key: &str, count: Option<usize>) -> RespType {
    if count == Some(0) {
        return RespType::Array(Some(vec![]));
    }
    let n = count.unwrap_or(1);
    with_db(|db| match db.get_mut(key) {
        Some(entry) => {
            if let Value::List(ref mut list) = entry.value {
                let mut popped: Vec<RespType> = Vec::new();
                for _ in 0..n {
                    match list.pop_front() {
                        Some(val) => popped.push(RespType::BulkString(Some(val))),
                        None => break,
                    }
                }
                entry.version = crate::db::bump_version();
                if list.is_empty() {
                    db.remove(key);
                }
                match count {
                    None => popped
                        .into_iter()
                        .next()
                        .unwrap_or(RespType::BulkString(None)),
                    Some(_) if popped.is_empty() => RespType::Array(None),
                    Some(_) => RespType::Array(Some(popped)),
                }
            } else {
                wrong_type()
            }
        }
        None => match count {
            None => RespType::BulkString(None),
            Some(_) => RespType::Array(None),
        },
    })
}

/// Try to pop from the first non-empty list among keys.
/// Returns `Some(RespType)` if we should respond (success or WRONGTYPE error).
/// Returns `None` if no data is available (caller should block).
pub fn try_blpop(keys: &[String]) -> Option<RespType> {
    with_db(|db| {
        for key in keys {
            match db.get_mut(key) {
                None => continue,
                Some(entry) => match &mut entry.value {
                    Value::List(list) => {
                        if let Some(val) = list.pop_front() {
                            entry.version = crate::db::bump_version();
                            if list.is_empty() {
                                db.remove(key);
                            }
                            return Some(RespType::Array(Some(vec![
                                RespType::BulkString(Some(Bytes::copy_from_slice(
                                    key.as_bytes(),
                                ))),
                                RespType::BulkString(Some(val)),
                            ])));
                        }
                    }
                    _ => return Some(wrong_type()),
                },
            }
        }
        None
    })
}

pub async fn handle_blpop(keys: &[String], timeout: u64) -> RespType {
    // First try — non-blocking
    if let Some(response) = try_blpop(keys) {
        return response;
    }

    // Blocking loop
    let notify = Arc::new(Notify::new());

    loop {
        let guard = with_db(|_| waiters::register(keys, &notify));

        if timeout == 0 {
            notify.notified().await;
        } else {
            let notified = notify.notified();
            tokio::pin!(notified);
            let timed_out = tokio::time::timeout(Duration::from_secs(timeout), notified)
                .await
                .is_err();
            if timed_out {
                drop(guard);
                return RespType::Array(None);
            }
        }

        drop(guard);

        match try_blpop(keys) {
            Some(response) => return response,
            None => continue,
        }
    }
}

pub fn handle_rpop(key: &str, count: Option<usize>) -> RespType {
    if count == Some(0) {
        return RespType::Array(Some(vec![]));
    }
    let n = count.unwrap_or(1);
    with_db(|db| match db.get_mut(key) {
        Some(entry) => {
            if let Value::List(ref mut list) = entry.value {
                let mut popped: Vec<RespType> = Vec::new();
                for _ in 0..n {
                    match list.pop_back() {
                        Some(val) => popped.push(RespType::BulkString(Some(val))),
                        None => break,
                    }
                }
                entry.version = crate::db::bump_version();
                if list.is_empty() {
                    db.remove(key);
                }
                match count {
                    None => popped
                        .into_iter()
                        .next()
                        .unwrap_or(RespType::BulkString(None)),
                    Some(_) if popped.is_empty() => RespType::Array(None),
                    Some(_) => RespType::Array(Some(popped)),
                }
            } else {
                wrong_type()
            }
        }
        None => match count {
            None => RespType::BulkString(None),
            Some(_) => RespType::Array(None),
        },
    })
}

pub fn handle_lindex(key: &str, index: i64) -> RespType {
    with_db(|db| match db.get(key) {
        Some(entry) => match &entry.value {
            Value::List(list) => {
                let len = list.len() as i64;
                let idx = if index < 0 { len + index } else { index };
                if idx < 0 || idx >= len {
                    RespType::BulkString(None)
                } else {
                    RespType::BulkString(Some(list[idx as usize].clone()))
                }
            }
            _ => wrong_type(),
        },
        None => RespType::BulkString(None),
    })
}

pub fn handle_lrem(key: &str, count: i64, value: &str) -> RespType {
    with_db(|db| match db.get_mut(key) {
        Some(entry) => {
            if let Value::List(ref mut list) = entry.value {
                let target = Bytes::from(value.to_string());
                let old_len = list.len();
                entry.version = crate::db::bump_version();
                if count > 0 {
                    let mut removed = 0i64;
                    let mut i = 0;
                    while i < list.len() && removed < count {
                        if list[i] == target {
                            list.remove(i);
                            removed += 1;
                        } else {
                            i += 1;
                        }
                    }
                    remove_key_if_empty(db, key);
                    RespType::Integer(removed)
                } else if count < 0 {
                    let limit = (-count) as usize;
                    let mut removed = 0;
                    let mut i = list.len();
                    while i > 0 && removed < limit {
                        i -= 1;
                        if list[i] == target {
                            list.remove(i);
                            removed += 1;
                        }
                    }
                    remove_key_if_empty(db, key);
                    RespType::Integer(removed as i64)
                } else {
                    let new_list: VecDeque<Bytes> = list
                        .iter()
                        .filter(|v| *v != &target)
                        .cloned()
                        .collect();
                    let removed = (old_len - new_list.len()) as i64;
                    *list = new_list;
                    remove_key_if_empty(db, key);
                    RespType::Integer(removed)
                }
            } else {
                wrong_type()
            }
        }
        None => RespType::Integer(0),
    })
}

fn remove_key_if_empty(db: &mut std::collections::HashMap<String, crate::db::Entry>, key: &str) {
    if db.get(key).is_some_and(|e| {
        if let Value::List(l) = &e.value {
            l.is_empty()
        } else {
            false
        }
    }) {
        db.remove(key);
    }
}

pub fn handle_ltrim(key: &str, start: i64, stop: i64) -> RespType {
    with_db(|db| match db.get_mut(key) {
        Some(entry) => {
            if let Value::List(ref mut list) = entry.value {
                let len = list.len() as i64;
                let mut l = if start < 0 { len + start } else { start };
                let mut r = if stop < 0 { len + stop } else { stop };
                if l < 0 {
                    l = 0;
                }
                if r >= len {
                    r = len - 1;
                }
                if l > r || len == 0 {
                    *list = VecDeque::new();
                } else {
                    let kept: VecDeque<Bytes> = list
                        .range(l as usize..=r as usize)
                        .cloned()
                        .collect();
                    *list = kept;
                }
                entry.version = crate::db::bump_version();
                remove_key_if_empty(db, key);
                RespType::SimpleString("OK".to_string())
            } else {
                wrong_type()
            }
        }
        None => RespType::SimpleString("OK".to_string()),
    })
}

pub fn handle_rpoplpush(source: &str, destination: &str) -> RespType {
    let val = with_db(|db| match db.get_mut(source) {
        Some(entry) => {
            if let Value::List(ref mut list) = entry.value {
                let popped = list.pop_back();
                entry.version = crate::db::bump_version();
                if list.is_empty() {
                    db.remove(source);
                }
                popped
            } else {
                None
            }
        }
        None => None,
    });
    match val {
        Some(v) => {
            with_db(|db| {
                let entry = db
                    .entry(destination.to_string())
                    .or_insert_with(|| Entry::new(Value::List(VecDeque::new()), None));
                match &mut entry.value {
                    Value::List(list) => {
                        list.push_front(v.clone());
                        entry.version = crate::db::bump_version();
                    }
                    _ => {}
                }
            });
            RespType::BulkString(Some(v))
        }
        None => RespType::BulkString(None),
    }
}

pub fn handle_lset(key: &str, index: i64, value: &str) -> RespType {
    with_db(|db| match db.get_mut(key) {
        Some(entry) => {
            if let Value::List(ref mut list) = entry.value {
                let len = list.len() as i64;
                let idx = if index < 0 { len + index } else { index };
                if idx < 0 || idx >= len {
                    RespType::Error("ERR index out of range".to_string())
                } else {
                    list[idx as usize] = Bytes::from(value.to_string());
                    entry.version = crate::db::bump_version();
                    RespType::SimpleString("OK".to_string())
                }
            } else {
                wrong_type()
            }
        }
        None => RespType::Error("ERR no such key".to_string()),
    })
}

fn wrong_type() -> RespType {
    resp::RespType::Error(
        "WRONGTYPE Operation against a key holding the wrong kind of value".to_string(),
    )
}
