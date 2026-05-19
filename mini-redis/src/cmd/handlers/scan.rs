use crate::storage::db::{Entry, Value, with_db};
use crate::protocol::resp::RespType;
use tokio::time::Instant;

fn glob_match(pattern: &str, s: &str) -> bool {
    // Simple glob: * matches any sequence, ? matches single char
    let mut pi = 0; // pattern index
    let mut si = 0; // string index
    let mut star_idx: Option<usize> = None;
    let mut match_idx: Option<usize> = None;
    let p = pattern.as_bytes();
    let s = s.as_bytes();

    while si < s.len() {
        if pi < p.len() && (p[pi] == b'?' || p[pi] == s[si]) {
            pi += 1;
            si += 1;
        } else if pi < p.len() && p[pi] == b'*' {
            star_idx = Some(pi);
            match_idx = Some(si);
            pi += 1;
        } else if let Some(si_backup) = star_idx {
            pi = si_backup + 1;
            if let Some(m) = &mut match_idx {
                *m += 1;
                si = *m;
            }
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == b'*' {
        pi += 1;
    }
    pi == p.len()
}

fn type_name(entry: &Entry) -> &'static str {
    match &entry.value {
        Value::String(_) => "string",
        Value::List(_) => "list",
        Value::Stream(_) => "stream",
        Value::Hash(_) => "hash",
        Value::Set(_) => "set",
        Value::ZSet(_) => "zset",
    }
}

fn is_expired(entry: &Entry) -> bool {
    entry.expiry.is_some_and(|exp| Instant::now() >= exp)
}

pub fn handle_scan(
    cursor: u64,
    match_pattern: Option<String>,
    count: u64,
    type_filter: Option<String>,
) -> RespType {
    with_db(|db| {
        let mut keys: Vec<&String> = db.keys().collect();
        keys.sort();

        let start = cursor as usize;
        let count = count.max(1) as usize;
        let mut results = Vec::new();
        let mut i = start;

        while results.len() < count && i < keys.len() {
            let key = keys[i];
            let entry = match db.get(key.as_str()) {
                Some(e) => e,
                None => {
                    i += 1;
                    continue;
                }
            };

            // Skip expired keys
            if is_expired(entry) {
                i += 1;
                continue;
            }

            // Apply MATCH filter
            if let Some(ref pat) = match_pattern {
                if !glob_match(pat, key) {
                    i += 1;
                    continue;
                }
            }

            // Apply TYPE filter
            if let Some(ref t) = type_filter {
                if !t.eq_ignore_ascii_case(type_name(entry)) {
                    i += 1;
                    continue;
                }
            }

            results.push(RespType::BulkString(Some(
                bytes::Bytes::copy_from_slice(key.as_bytes()),
            )));
            i += 1;
        }

        let next_cursor = if i >= keys.len() { 0u64 } else { i as u64 };

        RespType::Array(Some(vec![
            RespType::BulkString(Some(bytes::Bytes::copy_from_slice(
                next_cursor.to_string().as_bytes(),
            ))),
            RespType::Array(Some(results)),
        ]))
    })
}

pub fn handle_sscan(
    key: &str,
    cursor: u64,
    match_pattern: Option<String>,
    count: u64,
) -> RespType {
    with_db(|db| {
        let entry = match db.get(key) {
            Some(e) => e,
            None => {
                return RespType::Array(Some(vec![
                    RespType::BulkString(Some(bytes::Bytes::from_static(b"0"))),
                    RespType::Array(Some(vec![])),
                ]));
            }
        };
        let members = match &entry.value {
            Value::Set(s) => {
                let mut v: Vec<&bytes::Bytes> = s.iter().collect();
                v.sort();
                v
            }
            _ => return RespType::Error("WRONGTYPE".to_string()),
        };

        let start = cursor as usize;
        let count = count.max(1) as usize;
        let mut results = Vec::new();
        let mut i = start;

        while results.len() < count && i < members.len() {
            let member = String::from_utf8_lossy(members[i]);
            if let Some(ref pat) = match_pattern {
                if !glob_match(pat, &member) {
                    i += 1;
                    continue;
                }
            }
            results.push(RespType::BulkString(Some(members[i].clone())));
            i += 1;
        }

        let next_cursor = if i >= members.len() { 0u64 } else { i as u64 };
        RespType::Array(Some(vec![
            RespType::BulkString(Some(bytes::Bytes::copy_from_slice(
                next_cursor.to_string().as_bytes(),
            ))),
            RespType::Array(Some(results)),
        ]))
    })
}

pub fn handle_hscan(
    key: &str,
    cursor: u64,
    match_pattern: Option<String>,
    count: u64,
) -> RespType {
    with_db(|db| {
        let entry = match db.get(key) {
            Some(e) => e,
            None => {
                return RespType::Array(Some(vec![
                    RespType::BulkString(Some(bytes::Bytes::from_static(b"0"))),
                    RespType::Array(Some(vec![])),
                ]));
            }
        };
        let fields = match &entry.value {
            Value::Hash(h) => {
                let mut v: Vec<(&bytes::Bytes, &bytes::Bytes)> = h.iter().collect();
                v.sort_by(|a, b| a.0.cmp(b.0));
                v
            }
            _ => return RespType::Error("WRONGTYPE".to_string()),
        };

        let start = cursor as usize;
        let count = count.max(1) as usize;
        let mut results = Vec::new();
        let mut i = start;

        while results.len() < count * 2 && i < fields.len() {
            let fname = String::from_utf8_lossy(fields[i].0);
            if let Some(ref pat) = match_pattern {
                if !glob_match(pat, &fname) {
                    i += 1;
                    continue;
                }
            }
            results.push(RespType::BulkString(Some(fields[i].0.clone())));
            results.push(RespType::BulkString(Some(fields[i].1.clone())));
            i += 1;
        }

        let next_cursor = if i >= fields.len() { 0u64 } else { i as u64 };
        RespType::Array(Some(vec![
            RespType::BulkString(Some(bytes::Bytes::copy_from_slice(
                next_cursor.to_string().as_bytes(),
            ))),
            RespType::Array(Some(results)),
        ]))
    })
}

pub fn handle_zscan(
    key: &str,
    cursor: u64,
    match_pattern: Option<String>,
    count: u64,
) -> RespType {
    with_db(|db| {
        let entry = match db.get(key) {
            Some(e) => e,
            None => {
                return RespType::Array(Some(vec![
                    RespType::BulkString(Some(bytes::Bytes::from_static(b"0"))),
                    RespType::Array(Some(vec![])),
                ]));
            }
        };
        let members = match &entry.value {
            Value::ZSet(z) => {
                let mut v: Vec<(&i64, &bytes::Bytes)> = z.iter().map(|(s, m)| (s, m)).collect();
                v.sort_by(|a, b| a.1.cmp(b.1));
                v
            }
            _ => return RespType::Error("WRONGTYPE".to_string()),
        };

        let start = cursor as usize;
        let count = count.max(1) as usize;
        let mut results = Vec::new();
        let mut i = start;

        while results.len() < count * 2 && i < members.len() {
            let member = String::from_utf8_lossy(members[i].1);
            if let Some(ref pat) = match_pattern {
                if !glob_match(pat, &member) {
                    i += 1;
                    continue;
                }
            }
            results.push(RespType::BulkString(Some(members[i].1.clone())));
            results.push(RespType::Integer(*members[i].0));
            i += 1;
        }

        let next_cursor = if i >= members.len() { 0u64 } else { i as u64 };
        RespType::Array(Some(vec![
            RespType::BulkString(Some(bytes::Bytes::copy_from_slice(
                next_cursor.to_string().as_bytes(),
            ))),
            RespType::Array(Some(results)),
        ]))
    })
}
