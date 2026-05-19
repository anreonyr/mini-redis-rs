use crate::db::{with_db, Value};
use crate::resp::RespType;

pub fn handle_sadd(key: &str, members: &[String]) -> RespType {
    with_db(|db| {
        let entry = db.entry(key.to_string()).or_insert_with(|| {
            crate::db::Entry::new(Value::Set(std::collections::HashSet::new()), None)
        });
        match &mut entry.value {
            Value::Set(set) => {
                let mut new_count = 0i64;
                for m in members {
                    if set.insert(bytes::Bytes::copy_from_slice(m.as_bytes())) {
                        new_count += 1;
                    }
                }
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

fn wrong_type() -> RespType {
    RespType::Error(
        "WRONGTYPE Operation against a key holding the wrong kind of value".to_string(),
    )
}
