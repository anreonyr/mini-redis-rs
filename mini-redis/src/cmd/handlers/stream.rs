use bytes::Bytes;

use crate::cmd::types::XGroupSub;
use crate::db::{ConsumerGroup, Entry, StreamData, StreamEntry, Value, with_db};
use crate::resp;
use crate::resp::RespType;
use std::collections::HashMap;

// ── Stream ID helpers ─────────────────────────────────────────────────

fn parse_stream_id(id: &str) -> Option<(i64, u64)> {
    if id == "*" || id == "-" || id == "+" {
        return None;
    }
    let parts: Vec<&str> = id.splitn(2, '-').collect();
    if parts.len() != 2 {
        return None;
    }
    let ts = parts[0].parse::<i64>().ok()?;
    let seq = parts[1].parse::<u64>().ok()?;
    Some((ts, seq))
}

fn auto_stream_id(last_ts: i64, last_seq: u64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;
    let (ts, seq) = if now > last_ts {
        (now, 0)
    } else if now == last_ts {
        (last_ts, last_seq + 1)
    } else {
        (last_ts + 1, 0)
    };
    format!("{}-{}", ts, seq)
}

fn auto_seq_for_timestamp(ts: i64, last_ts: i64, last_seq: u64) -> String {
    if ts > last_ts {
        format!("{}-{}", ts, 0)
    } else if ts == last_ts {
        format!("{}-{}", ts, last_seq + 1)
    } else {
        format!("{}-0", last_ts + 1)
    }
}

fn make_stream_entry(id: String, fields: Vec<(Bytes, Bytes)>) -> RespType {
    let mut arr = vec![RespType::BulkString(Some(Bytes::from(id)))];
    let mut fv = Vec::with_capacity(fields.len() * 2);
    for (k, v) in &fields {
        fv.push(RespType::BulkString(Some(k.clone())));
        fv.push(RespType::BulkString(Some(v.clone())));
    }
    arr.push(RespType::Array(Some(fv)));
    RespType::Array(Some(arr))
}

// ── Stream command handlers ───────────────────────────────────────────

pub fn handle_xadd(key: &str, id_spec: &str, field_args: &[String]) -> RespType {
    let fields: Vec<(Bytes, Bytes)> = field_args
        .chunks(2)
        .map(|c| (Bytes::from(c[0].clone()), Bytes::from(c[1].clone())))
        .collect();

    with_db(|db| {
        let entry = db.entry(key.to_string()).or_insert_with(|| {
            Entry::new(Value::Stream(StreamData::new()), None)
        });

        match &mut entry.value {
            Value::Stream(stream) => {
                let is_empty = stream.entries.is_empty();
                let final_id = if id_spec == "*" {
                    auto_stream_id(stream.last_timestamp_ms, stream.last_seq)
                } else if let Some(ts) = id_spec.strip_suffix("-*") {
                    let t = ts.parse::<i64>().unwrap_or(0);
                    auto_seq_for_timestamp(t, stream.last_timestamp_ms, stream.last_seq)
                } else if let Some((ts, seq)) = parse_stream_id(id_spec) {
                    let last = (stream.last_timestamp_ms, stream.last_seq);
                    if !is_empty && (ts, seq) <= last {
                        return RespType::Error(
                            "ERR The ID specified in XADD is equal or smaller than the target stream top item".to_string(),
                        );
                    }
                    format!("{}-{}", ts, seq)
                } else {
                    return RespType::Error("ERR invalid stream ID".to_string());
                };

                if let Some((ts, seq)) = parse_stream_id(&final_id) {
                    stream.last_timestamp_ms = ts;
                    stream.last_seq = seq;
                }

                stream.entries.push_back(StreamEntry {
                    id: final_id.clone(),
                    fields,
                });

                entry.version = crate::db::bump_version();
                RespType::BulkString(Some(Bytes::from(final_id)))
            }
            _ => wrong_type(),
        }
    })
}

pub fn handle_xrange(key: &str, start: &str, end: &str, count: Option<u64>) -> RespType {
    with_db(|db| match db.get(key) {
        Some(entry) => match &entry.value {
            Value::Stream(stream) => {
                let start_id = if start == "-" {
                    (i64::MIN, 0u64)
                } else {
                    parse_stream_id(start).unwrap_or((i64::MIN, 0))
                };
                let end_id = if end == "+" {
                    (i64::MAX, u64::MAX)
                } else {
                    parse_stream_id(end).unwrap_or((i64::MAX, u64::MAX))
                };

                let matched: Vec<RespType> = stream
                    .entries
                    .iter()
                    .filter(|e| {
                        parse_stream_id(&e.id)
                            .map(|(ts, seq)| (ts, seq) >= start_id && (ts, seq) <= end_id)
                            .unwrap_or(false)
                    })
                    .take(count.unwrap_or(u64::MAX) as usize)
                    .map(|e| make_stream_entry(e.id.clone(), e.fields.clone()))
                    .collect();

                RespType::Array(Some(matched))
            }
            _ => wrong_type(),
        },
        None => RespType::Array(Some(vec![])),
    })
}

pub fn handle_xrevrange(key: &str, end: &str, start: &str, count: Option<u64>) -> RespType {
    with_db(|db| match db.get(key) {
        Some(entry) => match &entry.value {
            Value::Stream(stream) => {
                let end_id = if end == "+" {
                    (i64::MAX, u64::MAX)
                } else {
                    parse_stream_id(end).unwrap_or((i64::MAX, u64::MAX))
                };
                let start_id = if start == "-" {
                    (i64::MIN, 0u64)
                } else {
                    parse_stream_id(start).unwrap_or((i64::MIN, 0))
                };

                let matched: Vec<RespType> = stream
                    .entries
                    .iter()
                    .rev()
                    .filter(|e| {
                        parse_stream_id(&e.id)
                            .map(|(ts, seq)| (ts, seq) >= start_id && (ts, seq) <= end_id)
                            .unwrap_or(false)
                    })
                    .take(count.unwrap_or(u64::MAX) as usize)
                    .map(|e| make_stream_entry(e.id.clone(), e.fields.clone()))
                    .collect();

                RespType::Array(Some(matched))
            }
            _ => wrong_type(),
        },
        None => RespType::Array(Some(vec![])),
    })
}

pub fn handle_xlen(key: &str) -> RespType {
    with_db(|db| match db.get(key) {
        Some(entry) => match &entry.value {
            Value::Stream(stream) => RespType::Integer(stream.entries.len() as i64),
            _ => wrong_type(),
        },
        None => RespType::Integer(0),
    })
}

pub fn handle_xtrim(key: &str, _strategy: &str, threshold: u64, _exact: bool) -> RespType {
    with_db(|db| match db.get_mut(key) {
        Some(entry) => match &mut entry.value {
            Value::Stream(stream) => {
                let before = stream.entries.len();
                if before > threshold as usize {
                    let to_remove = before - threshold as usize;
                    stream.entries.drain(..to_remove);
                    entry.version = crate::db::bump_version();
                    if stream.entries.is_empty() {
                        stream.last_timestamp_ms = 0;
                        stream.last_seq = 0;
                    }
                    RespType::Integer(to_remove as i64)
                } else {
                    RespType::Integer(0)
                }
            }
            _ => wrong_type(),
        },
        None => RespType::Integer(0),
    })
}

pub fn handle_xdel(key: &str, ids: &[String]) -> RespType {
    with_db(|db| match db.get_mut(key) {
        Some(entry) => match &mut entry.value {
            Value::Stream(stream) => {
                let before = stream.entries.len();
                stream.entries.retain(|e| !ids.contains(&e.id));
                let removed = before - stream.entries.len();
                entry.version = crate::db::bump_version();
                if stream.entries.is_empty() {
                    db.remove(key);
                }
                RespType::Integer(removed as i64)
            }
            _ => wrong_type(),
        },
        None => RespType::Integer(0),
    })
}

pub fn handle_xread(count: Option<u64>, keys: &[String], ids: &[String]) -> RespType {
    with_db(|db| {
        let mut streams_resp: Vec<RespType> = Vec::new();

        for (key, since_id_str) in keys.iter().zip(ids.iter()) {
            let since = parse_stream_id(since_id_str).unwrap_or((0, 0));

            if let Some(entry) = db.get(key) {
                if let Value::Stream(ref stream) = entry.value {
                    let entries: Vec<RespType> = stream
                        .entries
                        .iter()
                        .filter(|e| {
                            parse_stream_id(&e.id)
                                .map(|(ts, seq)| (ts, seq) > since)
                                .unwrap_or(false)
                        })
                        .take(count.unwrap_or(u64::MAX) as usize)
                        .map(|e| make_stream_entry(e.id.clone(), e.fields.clone()))
                        .collect();

                    if !entries.is_empty() {
                        streams_resp.push(RespType::Array(Some(vec![
                            RespType::BulkString(Some(Bytes::from(key.clone()))),
                            RespType::Array(Some(entries)),
                        ])));
                    }
                }
            }
        }

        if streams_resp.is_empty() {
            RespType::Array(Some(vec![]))
        } else {
            RespType::Array(Some(streams_resp))
        }
    })
}

pub fn handle_xgroup(sub: XGroupSub, key: &str) -> RespType {
    with_db(|db| {
        let entry = db.get_mut(key);
        let entry = match entry {
            Some(e) => e,
            None => return RespType::Error("ERR no such stream key".to_string()),
        };
        let stream = match &mut entry.value {
            Value::Stream(s) => s,
            _ => return wrong_type(),
        };

        match sub {
            XGroupSub::Create { group, id } => {
                if stream.groups.contains_key(&group) {
                    return RespType::Error("BUSYGROUP Consumer Group name already exists".to_string());
                }
                // Validate the id
                if id != "$" && id != "0" && parse_stream_id(&id).is_none() {
                    return RespType::Error("ERR invalid stream ID".to_string());
                }
                // "$" means "last entry in stream", "0" means "from beginning"
                let last_delivered_id = if id == "$" {
                    stream.entries.back().map(|e| e.id.clone()).unwrap_or_else(|| "0-0".to_string())
                } else {
                    id.clone()
                };
                stream.groups.insert(group.clone(), ConsumerGroup {
                    name: group,
                    last_delivered_id,
                    pending: HashMap::new(),
                    consumers: HashMap::new(),
                });
                RespType::SimpleString("OK".to_string())
            }
            XGroupSub::Destroy { group } => {
                if stream.groups.remove(&group).is_some() {
                    RespType::Integer(1)
                } else {
                    RespType::Integer(0)
                }
            }
            XGroupSub::CreateConsumer { group, consumer } => {
                let cg = match stream.groups.get_mut(&group) {
                    Some(g) => g,
                    None => return RespType::Error("ERR no such consumer group".to_string()),
                };
                if cg.consumers.contains_key(&consumer) {
                    return RespType::Integer(0);
                }
                cg.consumers.insert(consumer.clone(), crate::db::ConsumerInfo {
                    name: consumer,
                    pending_count: 0,
                });
                RespType::Integer(1)
            }
            XGroupSub::DelConsumer { group, consumer } => {
                let cg = match stream.groups.get_mut(&group) {
                    Some(g) => g,
                    None => return RespType::Error("ERR no such consumer group".to_string()),
                };
                // Remove pending entries for this consumer
                let pending_count = cg.pending.remove(&consumer).map(|v| v.len() as i64).unwrap_or(0);
                cg.consumers.remove(&consumer);
                RespType::Integer(pending_count)
            }
            XGroupSub::SetId { group, id } => {
                let cg = match stream.groups.get_mut(&group) {
                    Some(g) => g,
                    None => return RespType::Error("ERR no such consumer group".to_string()),
                };
                cg.last_delivered_id = id;
                RespType::SimpleString("OK".to_string())
            }
        }
    })
}

pub fn handle_xreadgroup(
    group: &str,
    consumer: &str,
    count: Option<u64>,
    keys: &[String],
    ids: &[String],
) -> RespType {
    with_db(|db| {
        let mut streams_resp: Vec<RespType> = Vec::new();

        for (key, id_str) in keys.iter().zip(ids.iter()) {
            let entry = match db.get_mut(key) {
                Some(e) => e,
                None => continue,
            };
            let stream = match &mut entry.value {
                Value::Stream(s) => s,
                _ => continue,
            };
            let cg = match stream.groups.get_mut(group) {
                Some(g) => g,
                _ => continue,
            };

            let entries: Vec<RespType> = if id_str == ">" {
                // New messages: deliver from last_delivered_id
                let from_id = &cg.last_delivered_id;
                let from = parse_stream_id(from_id).unwrap_or((0, 0));

                let matched: Vec<(String, Vec<(Bytes, Bytes)>)> = stream
                    .entries
                    .iter()
                    .filter(|e| {
                        parse_stream_id(&e.id)
                            .map(|(ts, seq)| (ts, seq) > from)
                            .unwrap_or(false)
                    })
                    .take(count.unwrap_or(u64::MAX) as usize)
                    .map(|e| (e.id.clone(), e.fields.clone()))
                    .collect();

                // Mark as pending and update last_delivered_id
                for (id, _) in &matched {
                    cg.pending
                        .entry(consumer.to_string())
                        .or_default()
                        .push(crate::db::PendingEntry {
                            id: id.clone(),
                            consumer_name: consumer.to_string(),
                            delivery_count: 1,
                        });
                    let con = cg.consumers.entry(consumer.to_string()).or_insert_with(|| {
                        crate::db::ConsumerInfo { name: consumer.to_string(), pending_count: 0 }
                    });
                    con.pending_count += 1;
                }
                if let Some((last_id, _)) = matched.last() {
                    cg.last_delivered_id = last_id.clone();
                }

                matched
                    .into_iter()
                    .map(|(id, fields)| make_stream_entry(id, fields))
                    .collect()
            } else {
                // Read pending messages by ID
                let since = parse_stream_id(id_str).unwrap_or((0, 0));
                let pending = cg.pending.get(consumer).cloned().unwrap_or_default();
                pending
                    .into_iter()
                    .filter(|pe| {
                        parse_stream_id(&pe.id)
                            .map(|(ts, seq)| (ts, seq) > since)
                            .unwrap_or(false)
                    })
                    .take(count.unwrap_or(u64::MAX) as usize)
                    .filter_map(|pe| {
                        stream.entries.iter().find(|e| e.id == pe.id)
                            .map(|e| make_stream_entry(e.id.clone(), e.fields.clone()))
                    })
                    .collect()
            };

            if !entries.is_empty() {
                streams_resp.push(RespType::Array(Some(vec![
                    RespType::BulkString(Some(bytes::Bytes::from(key.clone()))),
                    RespType::Array(Some(entries)),
                ])));
            }
        }

        if streams_resp.is_empty() {
            RespType::Array(Some(vec![]))
        } else {
            RespType::Array(Some(streams_resp))
        }
    })
}

pub fn handle_xack(key: &str, group: &str, ids: &[String]) -> RespType {
    with_db(|db| {
        let entry = match db.get_mut(key) {
            Some(e) => e,
            None => return RespType::Integer(0),
        };
        let stream = match &mut entry.value {
            Value::Stream(s) => s,
            _ => return RespType::Integer(0),
        };
        let cg = match stream.groups.get_mut(group) {
            Some(g) => g,
            _ => return RespType::Integer(0),
        };

        let mut acked = 0i64;
        for id in ids {
            // Remove from pending across all consumers
            let mut found = false;
            for pending_list in cg.pending.values_mut() {
                if let Some(pos) = pending_list.iter().position(|pe| pe.id == *id) {
                    pending_list.swap_remove(pos);
                    found = true;
                    break;
                }
            }
            if found {
                acked += 1;
            }
        }
        RespType::Integer(acked)
    })
}

pub fn handle_xpending(
    key: &str,
    group: &str,
    start: &str,
    end: &str,
    count: u64,
    consumer: Option<&str>,
) -> RespType {
    with_db(|db| {
        let entry = match db.get(key) {
            Some(e) => e,
            None => return RespType::Array(Some(vec![])),
        };
        let stream = match &entry.value {
            Value::Stream(s) => s,
            _ => return RespType::Array(Some(vec![])),
        };
        let cg = match stream.groups.get(group) {
            Some(g) => g,
            _ => return RespType::Array(Some(vec![])),
        };

        let start_id = if start == "-" { (i64::MIN, 0u64) } else {
            parse_stream_id(start).unwrap_or((i64::MIN, 0))
        };
        let end_id = if end == "+" { (i64::MAX, u64::MAX) } else {
            parse_stream_id(end).unwrap_or((i64::MAX, u64::MAX))
        };

        let mut all_pending: Vec<&crate::db::PendingEntry> = Vec::new();
        for (con_name, entries) in &cg.pending {
            if let Some(consumer_name) = consumer {
                if con_name.as_str() != consumer_name {
                    continue;
                }
            }
            for pe in entries {
                if let Some((ts, seq)) = parse_stream_id(&pe.id) {
                    if (ts, seq) >= start_id && (ts, seq) <= end_id {
                        all_pending.push(pe);
                    }
                }
            }
        }
        all_pending.sort_by_key(|pe| pe.id.clone());
        all_pending.truncate(count as usize);

        let result: Vec<RespType> = all_pending.iter().map(|pe| {
            RespType::Array(Some(vec![
                RespType::BulkString(Some(bytes::Bytes::copy_from_slice(pe.id.as_bytes()))),
                RespType::BulkString(Some(bytes::Bytes::copy_from_slice(pe.consumer_name.as_bytes()))),
                RespType::Integer(pe.delivery_count as i64),
            ]))
        }).collect();

        RespType::Array(Some(result))
    })
}

pub fn handle_xclaim(
    key: &str,
    group: &str,
    consumer: &str,
    _min_idle: u64,
    ids: &[String],
) -> RespType {
    with_db(|db| {
        let entry = match db.get_mut(key) {
            Some(e) => e,
            None => return RespType::Array(Some(vec![])),
        };
        let stream = match &mut entry.value {
            Value::Stream(s) => s,
            _ => return RespType::Array(Some(vec![])),
        };
        let cg = match stream.groups.get_mut(group) {
            Some(g) => g,
            _ => return RespType::Array(Some(vec![])),
        };

        let mut claimed: Vec<RespType> = Vec::new();
        for id in ids {
            // Find the entry in the stream
            let stream_entry = match stream.entries.iter().find(|e| e.id == *id) {
                Some(e) => e.clone(),
                None => continue,
            };

            // Remove from old consumer's pending list
            for pending_list in cg.pending.values_mut() {
                pending_list.retain(|pe| pe.id != *id);
            }

            // Add to new consumer's pending list
            cg.pending
                .entry(consumer.to_string())
                .or_default()
                .push(crate::db::PendingEntry {
                    id: id.clone(),
                    consumer_name: consumer.to_string(),
                    delivery_count: 1,
                });

            // Ensure consumer exists
            cg.consumers.entry(consumer.to_string()).or_insert_with(|| {
                crate::db::ConsumerInfo { name: consumer.to_string(), pending_count: 0 }
            });

            claimed.push(make_stream_entry(stream_entry.id, stream_entry.fields));
        }

        RespType::Array(Some(claimed))
    })
}

pub fn handle_xinfo(sub: &str, key: &str, group: Option<&str>) -> RespType {
    with_db(|db| {
        let entry = match db.get(key) {
            Some(e) => e,
            None => return RespType::Error("ERR no such stream key".to_string()),
        };
        let stream = match &entry.value {
            Value::Stream(s) => s,
            _ => return RespType::Error("ERR no such stream key".to_string()),
        };

        match sub.to_uppercase().as_str() {
            "STREAM" => {
                let mut info = Vec::new();
                info.push(RespType::BulkString(Some(bytes::Bytes::copy_from_slice(b"length"))));
                info.push(RespType::Integer(stream.entries.len() as i64));
                info.push(RespType::BulkString(Some(bytes::Bytes::copy_from_slice(b"groups"))));
                info.push(RespType::Integer(stream.groups.len() as i64));
                RespType::Array(Some(info))
            }
            "GROUPS" => {
                let mut groups_info: Vec<RespType> = Vec::new();
                for cg in stream.groups.values() {
                    let total_pending: usize = cg.pending.values().map(|v| v.len()).sum();
                    groups_info.push(RespType::Array(Some(vec![
                        RespType::BulkString(Some(bytes::Bytes::copy_from_slice(b"name"))),
                        RespType::BulkString(Some(bytes::Bytes::copy_from_slice(cg.name.as_bytes()))),
                        RespType::BulkString(Some(bytes::Bytes::copy_from_slice(b"consumers"))),
                        RespType::Integer(cg.consumers.len() as i64),
                        RespType::BulkString(Some(bytes::Bytes::copy_from_slice(b"pending"))),
                        RespType::Integer(total_pending as i64),
                        RespType::BulkString(Some(bytes::Bytes::copy_from_slice(b"last-delivered-id"))),
                        RespType::BulkString(Some(bytes::Bytes::copy_from_slice(cg.last_delivered_id.as_bytes()))),
                    ])));
                }
                RespType::Array(Some(groups_info))
            }
            "CONSUMERS" => {
                let group_name = match group {
                    Some(g) => g,
                    None => return RespType::Error("ERR wrong number of arguments".to_string()),
                };
                let cg = match stream.groups.get(group_name) {
                    Some(g) => g,
                    None => return RespType::Error("ERR no such consumer group".to_string()),
                };
                let mut cons_info: Vec<RespType> = Vec::new();
                for con in cg.consumers.values() {
                    let pending = cg.pending.get(&con.name).map(|v| v.len() as i64).unwrap_or(0);
                    cons_info.push(RespType::Array(Some(vec![
                        RespType::BulkString(Some(bytes::Bytes::copy_from_slice(b"name"))),
                        RespType::BulkString(Some(bytes::Bytes::copy_from_slice(con.name.as_bytes()))),
                        RespType::BulkString(Some(bytes::Bytes::copy_from_slice(b"pending"))),
                        RespType::Integer(pending),
                    ])));
                }
                RespType::Array(Some(cons_info))
            }
            _ => RespType::Error("ERR unknown subcommand".to_string()),
        }
    })
}

fn wrong_type() -> RespType {
    resp::RespType::Error(
        "WRONGTYPE Operation against a key holding the wrong kind of value".to_string(),
    )
}
