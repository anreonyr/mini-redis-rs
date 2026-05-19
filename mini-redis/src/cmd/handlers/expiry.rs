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

pub fn handle_pexpire(key: &str, milliseconds: u64) -> RespType {
    with_db(|db| match db.get_mut(key) {
        Some(entry) => {
            if entry.expiry.is_some_and(|exp| Instant::now() >= exp) {
                RespType::Integer(0)
            } else {
                entry.expiry = Some(Instant::now() + Duration::from_millis(milliseconds));
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

pub fn handle_pttl(key: &str) -> RespType {
    with_db(|db| match db.get(key) {
        Some(entry) => {
            if let Some(exp) = entry.expiry {
                let now = Instant::now();
                if now >= exp {
                    RespType::Integer(-2)
                } else {
                    let ms = exp.saturating_duration_since(now).as_millis() as i64;
                    RespType::Integer(ms)
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

pub fn handle_expireat(key: &str, timestamp: u64) -> RespType {
    with_db(|db| match db.get_mut(key) {
        Some(entry) => {
            if entry.expiry.is_some_and(|exp| Instant::now() >= exp) {
                RespType::Integer(0)
            } else {
                let now_secs = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let diff = timestamp.saturating_sub(now_secs);
                entry.expiry = Some(Instant::now() + Duration::from_secs(diff));
                entry.version = crate::storage::db::bump_version();
                RespType::Integer(1)
            }
        }
        None => RespType::Integer(0),
    })
}

pub fn handle_pexpireat(key: &str, timestamp_ms: u64) -> RespType {
    with_db(|db| match db.get_mut(key) {
        Some(entry) => {
            if entry.expiry.is_some_and(|exp| Instant::now() >= exp) {
                RespType::Integer(0)
            } else {
                let now_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;
                let diff = timestamp_ms.saturating_sub(now_ms);
                entry.expiry = Some(Instant::now() + Duration::from_millis(diff));
                entry.version = crate::storage::db::bump_version();
                RespType::Integer(1)
            }
        }
        None => RespType::Integer(0),
    })
}

fn expiry_to_unix_secs(exp: Instant) -> u64 {
    let now = Instant::now();
    if exp <= now {
        return 0;
    }
    let remaining = exp.duration_since(now).as_secs();
    let now_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    now_unix + remaining
}

fn expiry_to_unix_ms(exp: Instant) -> u64 {
    let now = Instant::now();
    if exp <= now {
        return 0;
    }
    let remaining_ms = exp.duration_since(now).as_millis() as u64;
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    now_ms + remaining_ms
}

pub fn handle_expiretime(key: &str) -> RespType {
    with_db(|db| match db.get(key) {
        Some(entry) => {
            if let Some(exp) = entry.expiry {
                if Instant::now() >= exp {
                    RespType::Integer(-2)
                } else {
                    RespType::Integer(expiry_to_unix_secs(exp) as i64)
                }
            } else {
                RespType::Integer(-1)
            }
        }
        None => RespType::Integer(-2),
    })
}

pub fn handle_pexpiretime(key: &str) -> RespType {
    with_db(|db| match db.get(key) {
        Some(entry) => {
            if let Some(exp) = entry.expiry {
                if Instant::now() >= exp {
                    RespType::Integer(-2)
                } else {
                    RespType::Integer(expiry_to_unix_ms(exp) as i64)
                }
            } else {
                RespType::Integer(-1)
            }
        }
        None => RespType::Integer(-2),
    })
}
