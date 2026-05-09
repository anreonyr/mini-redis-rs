use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use tokio::sync::Notify;

use crate::blocking;
use crate::db::{Entry, Value, with_db};
use crate::resp;
use crate::resp::RespType;

pub fn handle_rpush(key: &str, values: &[String]) -> RespType {
    let values: VecDeque<Bytes> = values.iter().map(|v| Bytes::from(v.clone())).collect();
    let result = with_db(|db| match db.get_mut(key) {
        Some(entry) => {
            if let Value::List(ref mut list) = entry.value {
                list.extend(values);
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
    blocking::notify_waiters(key);
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
    blocking::notify_waiters(key);
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
        let guard = with_db(|_| blocking::register(keys, &notify));

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

fn wrong_type() -> RespType {
    resp::RespType::Error(
        "WRONGTYPE Operation against a key holding the wrong kind of value".to_string(),
    )
}
