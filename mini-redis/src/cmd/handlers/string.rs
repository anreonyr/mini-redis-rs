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

fn wrong_type() -> RespType {
    resp::RespType::Error(
        "WRONGTYPE Operation against a key holding the wrong kind of value".to_string(),
    )
}
