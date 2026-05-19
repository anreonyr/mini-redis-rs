use crate::storage::db::{with_db, Value};
use crate::protocol::resp::RespType;
use crate::server::waiters;
use bytes::Bytes;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;

pub fn handle_zadd(key: &str, members: &[(i64, String)]) -> RespType {
    let result = with_db(|db| {
        let entry = db.entry(key.to_string()).or_insert_with(|| {
            crate::storage::db::Entry::new(
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
                entry.version = crate::storage::db::bump_version();
                RespType::Integer(new_count)
            }
            _ => wrong_type(),
        }
    });
    waiters::notify_waiters(key);
    result
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
            crate::storage::db::Entry::new(Value::ZSet(std::collections::BTreeSet::new()), None)
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
                entry.version = crate::storage::db::bump_version();
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
                entry.version = crate::storage::db::bump_version();
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
                entry.version = crate::storage::db::bump_version();
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

// ── ZPOPMIN / ZPOPMAX ────────────────────────────────────────────

pub fn handle_zpopmin(key: &str, count: Option<usize>) -> RespType {
    let n = count.unwrap_or(1);
    with_db(|db| match db.get_mut(key) {
        Some(entry) => match &mut entry.value {
            Value::ZSet(set) => {
                let mut result = Vec::new();
                for _ in 0..n {
                    if let Some((score, member)) = set.pop_first() {
                        result.push(RespType::BulkString(Some(member)));
                        result.push(RespType::BulkString(Some(
                            bytes::Bytes::copy_from_slice(score.to_string().as_bytes()),
                        )));
                    } else {
                        break;
                    }
                }
                entry.version = crate::storage::db::bump_version();
                if set.is_empty() {
                    db.remove(key);
                }
                RespType::Array(Some(result))
            }
            _ => wrong_type(),
        },
        None => RespType::Array(Some(Vec::new())),
    })
}

pub fn handle_zpopmax(key: &str, count: Option<usize>) -> RespType {
    let n = count.unwrap_or(1);
    with_db(|db| match db.get_mut(key) {
        Some(entry) => match &mut entry.value {
            Value::ZSet(set) => {
                let mut result = Vec::new();
                for _ in 0..n {
                    if let Some((score, member)) = set.pop_last() {
                        result.push(RespType::BulkString(Some(member)));
                        result.push(RespType::BulkString(Some(
                            bytes::Bytes::copy_from_slice(score.to_string().as_bytes()),
                        )));
                    } else {
                        break;
                    }
                }
                entry.version = crate::storage::db::bump_version();
                if set.is_empty() {
                    db.remove(key);
                }
                RespType::Array(Some(result))
            }
            _ => wrong_type(),
        },
        None => RespType::Array(Some(Vec::new())),
    })
}

// ── BZPOPMIN / BZPOPMAX (blocking) ──────────────────────────────

pub fn try_bzpopmin(keys: &[String]) -> Option<RespType> {
    with_db(|db| {
        for key in keys {
            match db.get_mut(key) {
                None => continue,
                Some(entry) => match &mut entry.value {
                    Value::ZSet(set) => {
                        if let Some((score, member)) = set.pop_first() {
                            entry.version = crate::storage::db::bump_version();
                            if set.is_empty() {
                                db.remove(key);
                            }
                            return Some(RespType::Array(Some(vec![
                                RespType::BulkString(Some(Bytes::copy_from_slice(
                                    key.as_bytes(),
                                ))),
                                RespType::BulkString(Some(member)),
                                RespType::BulkString(Some(
                                    bytes::Bytes::copy_from_slice(
                                        score.to_string().as_bytes(),
                                    ),
                                )),
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

pub async fn handle_bzpopmin(keys: &[String], timeout: u64) -> RespType {
    // First try — non-blocking
    if let Some(response) = try_bzpopmin(keys) {
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
            let timed_out =
                tokio::time::timeout(Duration::from_secs(timeout), notified).await.is_err();
            if timed_out {
                drop(guard);
                return RespType::Array(None);
            }
        }

        drop(guard);

        match try_bzpopmin(keys) {
            Some(response) => return response,
            None => continue,
        }
    }
}

pub fn try_bzpopmax(keys: &[String]) -> Option<RespType> {
    with_db(|db| {
        for key in keys {
            match db.get_mut(key) {
                None => continue,
                Some(entry) => match &mut entry.value {
                    Value::ZSet(set) => {
                        if let Some((score, member)) = set.pop_last() {
                            entry.version = crate::storage::db::bump_version();
                            if set.is_empty() {
                                db.remove(key);
                            }
                            return Some(RespType::Array(Some(vec![
                                RespType::BulkString(Some(Bytes::copy_from_slice(
                                    key.as_bytes(),
                                ))),
                                RespType::BulkString(Some(member)),
                                RespType::BulkString(Some(
                                    bytes::Bytes::copy_from_slice(
                                        score.to_string().as_bytes(),
                                    ),
                                )),
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

pub async fn handle_bzpopmax(keys: &[String], timeout: u64) -> RespType {
    // First try — non-blocking
    if let Some(response) = try_bzpopmax(keys) {
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
            let timed_out =
                tokio::time::timeout(Duration::from_secs(timeout), notified).await.is_err();
            if timed_out {
                drop(guard);
                return RespType::Array(None);
            }
        }

        drop(guard);

        match try_bzpopmax(keys) {
            Some(response) => return response,
            None => continue,
        }
    }
}

fn wrong_type() -> RespType {
    RespType::Error(
        "WRONGTYPE Operation against a key holding the wrong kind of value".to_string(),
    )
}

// ── ZSet Set Operations (intersection / union / difference) ──────────────

/// Aggregate scores across keys by SUM/MIN/MAX with optional weights.
fn zset_aggregate(
    db: &HashMap<String, crate::storage::db::Entry>,
    keys: &[String],
    weights: &[f64],
    aggregate: &str,
) -> HashMap<Bytes, i64> {
    let mut scores: HashMap<Bytes, i64> = HashMap::new();

    for (i, key) in keys.iter().enumerate() {
        let weight = weights.get(i).copied().unwrap_or(1.0);
        if let Some(entry) = db.get(key) {
            if let Value::ZSet(zset) = &entry.value {
                for (score, member) in zset.iter() {
                    let weighted = (*score as f64 * weight) as i64;
                    match aggregate {
                        "MIN" => {
                            let entry = scores.entry(member.clone()).or_insert(i64::MAX);
                            *entry = (*entry).min(weighted);
                        }
                        "MAX" => {
                            let entry = scores.entry(member.clone()).or_insert(i64::MIN);
                            *entry = (*entry).max(weighted);
                        }
                        _ => {
                            // SUM
                            let entry = scores.entry(member.clone()).or_insert(0);
                            *entry += weighted;
                        }
                    }
                }
            }
        }
    }
    scores
}

pub fn handle_zinterstore(
    dest: &str,
    numkeys: usize,
    keys: &[String],
    weights: &[f64],
    aggregate: &str,
) -> RespType {
    with_db(|db| {
        // Collect members per key for intersection
        let mut member_sets: Vec<Vec<Bytes>> = Vec::new();
        for key in keys.iter().take(numkeys) {
            if let Some(entry) = db.get(key) {
                if let Value::ZSet(zset) = &entry.value {
                    let members: Vec<Bytes> = zset.iter().map(|(_, m)| m.clone()).collect();
                    member_sets.push(members);
                } else {
                    return wrong_type();
                }
            } else {
                member_sets.push(vec![]);
            }
        }

        if member_sets.is_empty() {
            return RespType::Integer(0);
        }

        // Find members that exist in ALL sets
        let mut intersection: Vec<Bytes> = Vec::new();
        'member: for member in &member_sets[0] {
            for set in &member_sets[1..] {
                if !set.contains(member) {
                    continue 'member;
                }
            }
            intersection.push(member.clone());
        }

        // Compute aggregated scores
        let mut result_zset = std::collections::BTreeSet::new();
        for member in &intersection {
            let mut score: i64 = 0;
            let mut first = true;
            for (i, key) in keys.iter().enumerate() {
                if let Some(entry) = db.get(key) {
                    if let Value::ZSet(zset) = &entry.value {
                        for (s, m) in zset.iter() {
                            if m == member {
                                let weight = weights.get(i).copied().unwrap_or(1.0);
                                let w = (*s as f64 * weight) as i64;
                                match aggregate {
                                    "MIN" => {
                                        score = if first { w } else { score.min(w) };
                                    }
                                    "MAX" => {
                                        score = if first { w } else { score.max(w) };
                                    }
                                    _ => {
                                        if first {
                                            score = w;
                                        } else {
                                            score += w;
                                        }
                                    }
                                }
                                first = false;
                                break;
                            }
                        }
                    }
                }
            }
            result_zset.insert((score, member.clone()));
        }

        // Store result in destination key
        let entry = db.entry(dest.to_string()).or_insert_with(|| {
            crate::storage::db::Entry::new(Value::ZSet(std::collections::BTreeSet::new()), None)
        });
        entry.value = Value::ZSet(result_zset.clone());
        entry.version = crate::storage::db::bump_version();
        RespType::Integer(result_zset.len() as i64)
    })
}

pub fn handle_zunionstore(
    dest: &str,
    numkeys: usize,
    keys: &[String],
    weights: &[f64],
    aggregate: &str,
) -> RespType {
    with_db(|db| {
        let scores = zset_aggregate(db, &keys[..numkeys], weights, aggregate);
        let mut result_zset = std::collections::BTreeSet::new();
        for (member, score) in scores {
            result_zset.insert((score, member));
        }
        let entry = db.entry(dest.to_string()).or_insert_with(|| {
            crate::storage::db::Entry::new(Value::ZSet(std::collections::BTreeSet::new()), None)
        });
        entry.value = Value::ZSet(result_zset.clone());
        entry.version = crate::storage::db::bump_version();
        RespType::Integer(result_zset.len() as i64)
    })
}

pub fn handle_zinter(
    numkeys: usize,
    keys: &[String],
    weights: &[f64],
    aggregate: &str,
    withscores: bool,
) -> RespType {
    with_db(|db| {
        // Collect members per key for intersection
        let mut member_sets: Vec<Vec<Bytes>> = Vec::new();
        for key in keys.iter().take(numkeys) {
            if let Some(entry) = db.get(key) {
                if let Value::ZSet(zset) = &entry.value {
                    member_sets.push(zset.iter().map(|(_, m)| m.clone()).collect());
                } else {
                    return wrong_type();
                }
            } else {
                member_sets.push(vec![]);
            }
        }
        if member_sets.is_empty() {
            return RespType::Array(Some(vec![]));
        }
        let mut intersection: Vec<Bytes> = Vec::new();
        'member: for member in &member_sets[0] {
            for set in &member_sets[1..] {
                if !set.contains(member) {
                    continue 'member;
                }
            }
            intersection.push(member.clone());
        }
        let mut result_zset = std::collections::BTreeSet::new();
        for member in &intersection {
            let mut score: i64 = 0;
            let mut first = true;
            for (i, key) in keys.iter().enumerate() {
                if let Some(entry) = db.get(key) {
                    if let Value::ZSet(zset) = &entry.value {
                        for (s, m) in zset.iter() {
                            if m == member {
                                let w = weights.get(i).copied().unwrap_or(1.0);
                                let ws = (*s as f64 * w) as i64;
                                match aggregate {
                                    "MIN" => {
                                        score = if first { ws } else { score.min(ws) };
                                    }
                                    "MAX" => {
                                        score = if first { ws } else { score.max(ws) };
                                    }
                                    _ => {
                                        if first {
                                            score = ws;
                                        } else {
                                            score += ws;
                                        }
                                    }
                                }
                                first = false;
                                break;
                            }
                        }
                    }
                }
            }
            result_zset.insert((score, member.clone()));
        }
        let result: Vec<RespType> = result_zset
            .iter()
            .flat_map(|(s, m)| {
                if withscores {
                    vec![
                        RespType::BulkString(Some(m.clone())),
                        RespType::BulkString(Some(
                            bytes::Bytes::copy_from_slice(s.to_string().as_bytes()),
                        )),
                    ]
                } else {
                    vec![RespType::BulkString(Some(m.clone()))]
                }
            })
            .collect();
        RespType::Array(Some(result))
    })
}

pub fn handle_zunion(
    numkeys: usize,
    keys: &[String],
    weights: &[f64],
    aggregate: &str,
    withscores: bool,
) -> RespType {
    with_db(|db| {
        let scores = zset_aggregate(db, &keys[..numkeys], weights, aggregate);
        let mut result_zset = std::collections::BTreeSet::new();
        for (member, score) in scores {
            result_zset.insert((score, member));
        }
        let result: Vec<RespType> = result_zset
            .iter()
            .flat_map(|(s, m)| {
                if withscores {
                    vec![
                        RespType::BulkString(Some(m.clone())),
                        RespType::BulkString(Some(
                            bytes::Bytes::copy_from_slice(s.to_string().as_bytes()),
                        )),
                    ]
                } else {
                    vec![RespType::BulkString(Some(m.clone()))]
                }
            })
            .collect();
        RespType::Array(Some(result))
    })
}

pub fn handle_zdiff(keys: &[String], withscores: bool) -> RespType {
    with_db(|db| {
        if keys.is_empty() || !db.contains_key(&keys[0]) {
            return RespType::Array(Some(vec![]));
        }
        let first_set = match &db[&keys[0]].value {
            Value::ZSet(z) => z.clone(),
            _ => return wrong_type(),
        };
        // Collect members from all other sets
        let other_members: std::collections::HashSet<Bytes> = keys[1..]
            .iter()
            .filter_map(|k| db.get(k))
            .filter_map(|e| match &e.value {
                Value::ZSet(z) => {
                    Some(z.iter().map(|(_, m)| m.clone()).collect::<std::collections::HashSet<_>>())
                }
                _ => None,
            })
            .flatten()
            .collect();

        let result: Vec<RespType> = first_set
            .iter()
            .filter(|(_, m)| !other_members.contains(m))
            .flat_map(|(s, m)| {
                if withscores {
                    vec![
                        RespType::BulkString(Some(m.clone())),
                        RespType::BulkString(Some(
                            bytes::Bytes::copy_from_slice(s.to_string().as_bytes()),
                        )),
                    ]
                } else {
                    vec![RespType::BulkString(Some(m.clone()))]
                }
            })
            .collect();
        RespType::Array(Some(result))
    })
}

pub fn handle_zdiffstore(dest: &str, keys: &[String]) -> RespType {
    with_db(|db| {
        if keys.is_empty() || !db.contains_key(&keys[0]) {
            let entry = db.entry(dest.to_string()).or_insert_with(|| {
                crate::storage::db::Entry::new(Value::ZSet(std::collections::BTreeSet::new()), None)
            });
            entry.value = Value::ZSet(std::collections::BTreeSet::new());
            entry.version = crate::storage::db::bump_version();
            return RespType::Integer(0);
        }
        let first_set = match &db[&keys[0]].value {
            Value::ZSet(z) => z.clone(),
            _ => return wrong_type(),
        };
        let other_members: std::collections::HashSet<Bytes> = keys[1..]
            .iter()
            .filter_map(|k| db.get(k))
            .filter_map(|e| match &e.value {
                Value::ZSet(z) => {
                    Some(z.iter().map(|(_, m)| m.clone()).collect::<std::collections::HashSet<_>>())
                }
                _ => None,
            })
            .flatten()
            .collect();

        let mut result_zset = std::collections::BTreeSet::new();
        for (score, member) in first_set {
            if !other_members.contains(&member) {
                result_zset.insert((score, member));
            }
        }
        let count = result_zset.len();
        let entry = db.entry(dest.to_string()).or_insert_with(|| {
            crate::storage::db::Entry::new(Value::ZSet(std::collections::BTreeSet::new()), None)
        });
        entry.value = Value::ZSet(result_zset);
        entry.version = crate::storage::db::bump_version();
        RespType::Integer(count as i64)
    })
}
