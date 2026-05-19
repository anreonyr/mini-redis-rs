use crate::db::{with_db, Value};
use crate::resp::RespType;

pub fn handle_zadd(key: &str, members: &[(i64, String)]) -> RespType {
    with_db(|db| {
        let entry = db.entry(key.to_string()).or_insert_with(|| {
            crate::db::Entry::new(
                Value::ZSet(std::collections::BTreeSet::new()),
                None,
            )
        });
        match &mut entry.value {
            Value::ZSet(set) => {
                let mut new_count = 0i64;
                for (score, member) in members {
                    let member_bytes = bytes::Bytes::copy_from_slice(member.as_bytes());
                    let tuple = (*score, member_bytes.clone());
                    let existing = set
                        .iter()
                        .find(|(_, m)| m == &member_bytes)
                        .cloned();
                    if let Some(old) = existing {
                        set.remove(&old);
                        set.insert(tuple);
                    } else {
                        set.insert(tuple);
                        new_count += 1;
                    }
                }
                RespType::Integer(new_count)
            }
            _ => wrong_type(),
        }
    })
}

pub fn handle_zrange(key: &str, start: i64, stop: i64, withscores: bool) -> RespType {
    with_db(|db| match db.get(key) {
        Some(entry) => match &entry.value {
            Value::ZSet(set) => {
                if set.is_empty() {
                    return RespType::Array(Some(Vec::new()));
                }
                let items: Vec<&(i64, bytes::Bytes)> = set.iter().collect();
                let len = items.len() as i64;
                let start = if start < 0 { (len + start).max(0) } else { start.min(len - 1) };
                let stop = if stop < 0 { (len + stop).max(0) } else { stop.min(len - 1) };
                if start > stop || start >= len {
                    return RespType::Array(Some(Vec::new()));
                }
                let end = (stop + 1).min(len);
                let slice = &items[start as usize..end as usize];
                if withscores {
                    let mut result = Vec::with_capacity(slice.len() * 2);
                    for (score, member) in slice {
                        result.push(RespType::BulkString(Some((*member).clone())));
                        result.push(RespType::BulkString(Some(
                            bytes::Bytes::copy_from_slice(score.to_string().as_bytes()),
                        )));
                    }
                    RespType::Array(Some(result))
                } else {
                    let result: Vec<RespType> = slice
                        .iter()
                        .map(|(_, m)| RespType::BulkString(Some((*m).clone())))
                        .collect();
                    RespType::Array(Some(result))
                }
            }
            _ => wrong_type(),
        },
        None => RespType::Array(Some(Vec::new())),
    })
}

pub fn handle_zrank(key: &str, member: &str) -> RespType {
    with_db(|db| match db.get(key) {
        Some(entry) => match &entry.value {
            Value::ZSet(set) => {
                let mb = bytes::Bytes::copy_from_slice(member.as_bytes());
                for (rank, (_, m)) in set.iter().enumerate() {
                    if m == &mb {
                        return RespType::Integer(rank as i64);
                    }
                }
                RespType::BulkString(None)
            }
            _ => wrong_type(),
        },
        None => RespType::BulkString(None),
    })
}

pub fn handle_zscore(key: &str, member: &str) -> RespType {
    with_db(|db| match db.get(key) {
        Some(entry) => match &entry.value {
            Value::ZSet(set) => {
                let mb = bytes::Bytes::copy_from_slice(member.as_bytes());
                for (score, m) in set.iter() {
                    if m == &mb {
                        return RespType::BulkString(Some(bytes::Bytes::copy_from_slice(
                            score.to_string().as_bytes(),
                        )));
                    }
                }
                RespType::BulkString(None)
            }
            _ => wrong_type(),
        },
        None => RespType::BulkString(None),
    })
}

fn wrong_type() -> RespType {
    RespType::Error(
        "WRONGTYPE Operation against a key holding the wrong kind of value".to_string(),
    )
}
