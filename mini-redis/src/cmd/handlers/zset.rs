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

pub fn handle_zrem(key: &str, members: &[String]) -> RespType {
    with_db(|db| match db.get_mut(key) {
        Some(entry) => match &mut entry.value {
            Value::ZSet(set) => {
                let mut removed = 0i64;
                for member in members {
                    let mb = bytes::Bytes::copy_from_slice(member.as_bytes());
                    let to_remove = set.iter().find(|(_, m)| m == &mb).cloned();
                    if let Some(tuple) = to_remove {
                        set.remove(&tuple);
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

pub fn handle_zcard(key: &str) -> RespType {
    with_db(|db| match db.get(key) {
        Some(entry) => match &entry.value {
            Value::ZSet(set) => RespType::Integer(set.len() as i64),
            _ => wrong_type(),
        },
        None => RespType::Integer(0),
    })
}

fn parse_score_bound(s: &str) -> (i64, bool) {
    match s {
        "-inf" => (i64::MIN, true),
        "+inf" => (i64::MAX, true),
        s if s.starts_with('(') => {
            let v: i64 = s[1..].parse().unwrap_or(0);
            (v, false)
        }
        _ => {
            let v: i64 = s.parse().unwrap_or(0);
            (v, true)
        }
    }
}

pub fn handle_zcount(key: &str, min: &str, max: &str) -> RespType {
    let (min_s, min_inc) = parse_score_bound(min);
    let (max_s, max_inc) = parse_score_bound(max);
    with_db(|db| match db.get(key) {
        Some(entry) => match &entry.value {
            Value::ZSet(set) => {
                let count = set
                    .iter()
                    .filter(|(score, _)| {
                        let above_min = if min_inc { *score >= min_s } else { *score > min_s };
                        let below_max = if max_inc { *score <= max_s } else { *score < max_s };
                        above_min && below_max
                    })
                    .count() as i64;
                RespType::Integer(count)
            }
            _ => wrong_type(),
        },
        None => RespType::Integer(0),
    })
}

pub fn handle_zrangebyscore(
    key: &str,
    min: &str,
    max: &str,
    withscores: bool,
    limit: Option<(usize, usize)>,
) -> RespType {
    let (min_s, min_inc) = parse_score_bound(min);
    let (max_s, max_inc) = parse_score_bound(max);
    with_db(|db| match db.get(key) {
        Some(entry) => match &entry.value {
            Value::ZSet(set) => {
                let filtered: Vec<&(i64, bytes::Bytes)> = set
                    .iter()
                    .filter(|(score, _)| {
                        let above_min = if min_inc { *score >= min_s } else { *score > min_s };
                        let below_max = if max_inc { *score <= max_s } else { *score < max_s };
                        above_min && below_max
                    })
                    .collect();
                let slice = if let Some((offset, count)) = limit {
                    let start = offset.min(filtered.len());
                    let end = (start + count).min(filtered.len());
                    &filtered[start..end]
                } else {
                    &filtered
                };
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

pub fn handle_zincrby(key: &str, incr: i64, member: &str) -> RespType {
    with_db(|db| {
        let entry = db.entry(key.to_string()).or_insert_with(|| {
            crate::db::Entry::new(Value::ZSet(std::collections::BTreeSet::new()), None)
        });
        match &mut entry.value {
            Value::ZSet(set) => {
                let mb = bytes::Bytes::copy_from_slice(member.as_bytes());
                let existing = set.iter().find(|(_, m)| m == &mb).cloned();
                let new_score = if let Some((old_score, _)) = existing {
                    set.remove(&(old_score, mb.clone()));
                    old_score + incr
                } else {
                    incr
                };
                set.insert((new_score, mb));
                RespType::BulkString(Some(bytes::Bytes::copy_from_slice(
                    new_score.to_string().as_bytes(),
                )))
            }
            _ => wrong_type(),
        }
    })
}

pub fn handle_zrevrange(key: &str, start: i64, stop: i64, withscores: bool) -> RespType {
    with_db(|db| match db.get(key) {
        Some(entry) => match &entry.value {
            Value::ZSet(set) => {
                if set.is_empty() {
                    return RespType::Array(Some(Vec::new()));
                }
                let items: Vec<&(i64, bytes::Bytes)> = set.iter().rev().collect();
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

pub fn handle_zrevrank(key: &str, member: &str) -> RespType {
    with_db(|db| match db.get(key) {
        Some(entry) => match &entry.value {
            Value::ZSet(set) => {
                let mb = bytes::Bytes::copy_from_slice(member.as_bytes());
                for (rank, (_, m)) in set.iter().rev().enumerate() {
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

pub fn handle_zremrangebyrank(key: &str, start: i64, stop: i64) -> RespType {
    with_db(|db| match db.get_mut(key) {
        Some(entry) => match &mut entry.value {
            Value::ZSet(set) => {
                let items: Vec<&(i64, bytes::Bytes)> = set.iter().collect();
                let len = items.len() as i64;
                let s = if start < 0 { (len + start).max(0) } else { start.min(len) };
                let e = if stop < 0 { (len + stop).max(-1) } else { stop.min(len - 1) };
                if s > e || s >= len {
                    return RespType::Integer(0);
                }
                let to_remove: Vec<(i64, bytes::Bytes)> = items
                    [s as usize..=e as usize]
                    .iter()
                    .map(|t| (*t).clone())
                    .collect();
                for t in &to_remove {
                    set.remove(t);
                }
                if set.is_empty() {
                    db.remove(key);
                }
                RespType::Integer(to_remove.len() as i64)
            }
            _ => wrong_type(),
        },
        None => RespType::Integer(0),
    })
}

pub fn handle_zremrangebyscore(key: &str, min: &str, max: &str) -> RespType {
    let (min_s, min_inc) = parse_score_bound(min);
    let (max_s, max_inc) = parse_score_bound(max);
    with_db(|db| match db.get_mut(key) {
        Some(entry) => match &mut entry.value {
            Value::ZSet(set) => {
                let to_remove: Vec<(i64, bytes::Bytes)> = set
                    .iter()
                    .filter(|(score, _)| {
                        let above_min = if min_inc { *score >= min_s } else { *score > min_s };
                        let below_max = if max_inc { *score <= max_s } else { *score < max_s };
                        above_min && below_max
                    })
                    .cloned()
                    .collect();
                let count = to_remove.len() as i64;
                for t in &to_remove {
                    set.remove(t);
                }
                if set.is_empty() {
                    db.remove(key);
                }
                RespType::Integer(count)
            }
            _ => wrong_type(),
        },
        None => RespType::Integer(0),
    })
}

pub fn handle_zrevrangebyscore(
    key: &str,
    max: &str,
    min: &str,
    withscores: bool,
    limit: Option<(usize, usize)>,
) -> RespType {
    let (min_s, min_inc) = parse_score_bound(min);
    let (max_s, max_inc) = parse_score_bound(max);
    with_db(|db| match db.get(key) {
        Some(entry) => match &entry.value {
            Value::ZSet(set) => {
                let filtered: Vec<&(i64, bytes::Bytes)> = set
                    .iter()
                    .rev()
                    .filter(|(score, _)| {
                        let above_min = if min_inc { *score >= min_s } else { *score > min_s };
                        let below_max = if max_inc { *score <= max_s } else { *score < max_s };
                        above_min && below_max
                    })
                    .collect();
                let slice = if let Some((offset, count)) = limit {
                    let start = offset.min(filtered.len());
                    let end = (start + count).min(filtered.len());
                    &filtered[start..end]
                } else {
                    &filtered
                };
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

fn wrong_type() -> RespType {
    RespType::Error(
        "WRONGTYPE Operation against a key holding the wrong kind of value".to_string(),
    )
}
