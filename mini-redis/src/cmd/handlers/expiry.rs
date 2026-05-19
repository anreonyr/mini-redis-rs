use std::time::Duration;
use tokio::time::Instant;

use crate::storage::db::with_db;
use crate::protocol::resp::RespType;

pub fn handle_expire(key: &str, seconds: u64) -> RespType {
    with_db(|db| match db.get_mut(key) {
        Some(entry) => {
            if entry.expiry.is_some_and(|exp| Instant::now() >= exp) {
                RespType::Integer(0)
            } else {
                entry.expiry = Some(Instant::now() + Duration::from_secs(seconds));
                entry.version = crate::storage::db::bump_version();
                RespType::Integer(1)
            }
        }
        None => RespType::Integer(0),
    })
}

pub fn handle_ttl(key: &str) -> RespType {
    with_db(|db| match db.get(key) {
        Some(entry) => {
            if let Some(exp) = entry.expiry {
                let now = Instant::now();
                if now >= exp {
                    RespType::Integer(-2)
                } else {
                    let remaining_ms = exp.saturating_duration_since(now).as_millis() as f64;
                    let secs = (remaining_ms / 1000.0).ceil() as i64;
                    RespType::Integer(secs)
                }
            } else {
                RespType::Integer(-1)
            }
        }
        None => RespType::Integer(-2),
    })
}

pub fn handle_persist(key: &str) -> RespType {
    with_db(|db| match db.get_mut(key) {
        Some(entry) => {
            if entry.expiry.is_some_and(|exp| Instant::now() >= exp) {
                RespType::Integer(0)
            } else if entry.expiry.is_some() {
                entry.expiry = None;
                entry.version = crate::storage::db::bump_version();
                RespType::Integer(1)
            } else {
                RespType::Integer(0)
            }
        }
        None => RespType::Integer(0),
    })
}
