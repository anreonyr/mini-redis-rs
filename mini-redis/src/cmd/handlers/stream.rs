use bytes::Bytes;

use crate::db::{Entry, StreamData, StreamEntry, Value, with_db};
use crate::resp;
use crate::resp::RespType;

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

fn wrong_type() -> RespType {
    resp::RespType::Error(
        "WRONGTYPE Operation against a key holding the wrong kind of value".to_string(),
    )
}
