use bytes::Bytes;
use tokio::time::Instant;

use crate::cmd::types::BitFieldSub;
use crate::storage::db::{bump_version, Entry, Value, with_db};
use crate::protocol::resp;
use crate::protocol::resp::RespType;
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

pub fn handle_incr(key: &str) -> RespType {
    incrby(key, 1)
}

pub fn handle_decr(key: &str) -> RespType {
    incrby(key, -1)
}

pub fn handle_incrby(key: &str, delta: i64) -> RespType {
    incrby(key, delta)
}

pub fn handle_decrby(key: &str, delta: i64) -> RespType {
    incrby(key, -delta)
}

fn incrby(key: &str, delta: i64) -> RespType {
    with_db(|db| {
        let entry = db
            .entry(key.to_string())
            .and_modify(|e| {
                if e.expiry.is_some_and(|exp| Instant::now() >= exp) {
                    e.expiry = None;
                    e.value = Value::String(Bytes::from("0"));
                }
            })
            .or_insert_with(|| Entry::new(Value::String(Bytes::from("0")), None));

        match &entry.value {
            Value::String(v) => {
                let current: i64 = match std::str::from_utf8(v).ok().and_then(|s| s.parse().ok()) {
                    Some(n) => n,
                    None => {
                        return RespType::Error(
                            "ERR value is not an integer or out of range".to_string(),
                        );
                    }
                };
                let new_val = current.wrapping_add(delta);
                entry.value = Value::String(Bytes::from(new_val.to_string()));
                entry.version = bump_version();
                RespType::Integer(new_val)
            }
            _ => wrong_type(),
        }
    })
}

pub fn handle_append(key: &str, value: &str) -> RespType {
    with_db(|db| {
        let entry = db.entry(key.to_string()).or_insert_with(|| {
            Entry::new(Value::String(Bytes::new()), None)
        });
        match &mut entry.value {
            Value::String(v) => {
                let mut data = v.to_vec();
                data.extend_from_slice(value.as_bytes());
                *v = Bytes::from(data);
                entry.version = bump_version();
                RespType::Integer(v.len() as i64)
            }
            _ => wrong_type(),
        }
    })
}

pub fn handle_strlen(key: &str) -> RespType {
    with_db(|db| match db.get(key) {
        Some(entry) => {
            if entry.expiry.is_some_and(|exp| Instant::now() >= exp) {
                db.remove(key);
                return RespType::Integer(0);
            }
            match &entry.value {
                Value::String(v) => RespType::Integer(v.len() as i64),
                _ => wrong_type(),
            }
        }
        None => RespType::Integer(0),
    })
}

pub fn handle_mget(keys: &[String]) -> RespType {
    with_db(|db| {
        let results: Vec<RespType> = keys
            .iter()
            .map(|key| match db.get(key) {
                Some(entry) => {
                    if entry.expiry.is_some_and(|exp| Instant::now() >= exp) {
                        RespType::BulkString(None)
                    } else {
                        match &entry.value {
                            Value::String(v) => RespType::BulkString(Some(v.clone())),
                            _ => RespType::BulkString(None),
                        }
                    }
                }
                None => RespType::BulkString(None),
            })
            .collect();
        RespType::Array(Some(results))
    })
}

pub fn handle_mset(pairs: &[(String, String)]) -> RespType {
    with_db(|db| {
        for (key, value) in pairs {
            db.insert(
                key.clone(),
                Entry::new(Value::String(Bytes::from(value.clone())), None),
            );
        }
    });
    RespType::SimpleString("OK".to_string())
}

pub fn handle_getset(key: &str, value: &str) -> RespType {
    with_db(|db| {
        let old = db.get(key).and_then(|entry| {
            if entry.expiry.is_some_and(|exp| Instant::now() >= exp) {
                None
            } else {
                match &entry.value {
                    Value::String(v) => Some(v.clone()),
                    _ => None,
                }
            }
        });
        if let Some(old_val) = old {
            db.insert(
                key.to_string(),
                Entry::new(Value::String(Bytes::from(value.to_string())), None),
            );
            RespType::BulkString(Some(old_val))
        } else if db.contains_key(key) {
            // wrong type
            wrong_type()
        } else {
            RespType::BulkString(None)
        }
    })
}

pub fn handle_getrange(key: &str, start: i64, end: i64) -> RespType {
    with_db(|db| match db.get(key) {
        Some(entry) => {
            if entry.expiry.is_some_and(|exp| Instant::now() >= exp) {
                db.remove(key);
                return RespType::BulkString(Some(Bytes::new()));
            }
            match &entry.value {
                Value::String(v) => {
                    let len = v.len() as i64;
                    if len == 0 {
                        return RespType::BulkString(Some(Bytes::new()));
                    }
                    let s = if start < 0 { (len + start).max(0) } else { start.min(len - 1) };
                    let e = if end < 0 { (len + end).max(0) } else { end.min(len - 1) };
                    if s > e || s >= len {
                        RespType::BulkString(Some(Bytes::new()))
                    } else {
                        let slice = v.slice(s as usize..(e + 1) as usize);
                        RespType::BulkString(Some(slice))
                    }
                }
                _ => wrong_type(),
            }
        }
        None => RespType::BulkString(Some(Bytes::new())),
    })
}

pub fn handle_setrange(key: &str, offset: u64, value: &str) -> RespType {
    with_db(|db| {
        let entry = db.entry(key.to_string()).or_insert_with(|| {
            Entry::new(Value::String(Bytes::new()), None)
        });
        match &mut entry.value {
            Value::String(v) => {
                let off = offset as usize;
                let val_bytes = value.as_bytes();
                let needed = off + val_bytes.len();
                if needed > v.len() {
                    let mut new_data = v.to_vec();
                    new_data.resize(needed, 0);
                    new_data[off..off + val_bytes.len()].copy_from_slice(val_bytes);
                    *v = Bytes::from(new_data);
                } else {
                    let mut new_data = v.to_vec();
                    new_data[off..off + val_bytes.len()].copy_from_slice(val_bytes);
                    *v = Bytes::from(new_data);
                }
                entry.version = bump_version();
                RespType::Integer(v.len() as i64)
            }
            _ => wrong_type(),
        }
    })
}

pub fn handle_msetnx(pairs: &[(String, String)]) -> RespType {
    with_db(|db| {
        for (key, _) in pairs {
            if db.contains_key(key) {
                return RespType::Integer(0);
            }
        }
        for (key, value) in pairs {
            db.insert(
                key.clone(),
                Entry::new(Value::String(Bytes::from(value.clone())), None),
            );
        }
        RespType::Integer(1)
    })
}

pub fn handle_setnx(key: &str, value: &str) -> RespType {
    with_db(|db| {
        if db.contains_key(key) {
            return RespType::Integer(0);
        }
        db.insert(
            key.to_string(),
            Entry::new(Value::String(Bytes::from(value.to_string())), None),
        );
        RespType::Integer(1)
    })
}

pub fn handle_getex(key: &str, expiry: Option<Duration>) -> RespType {
    with_db(|db| match db.get_mut(key) {
        Some(entry) => {
            if entry.expiry.is_some_and(|exp| Instant::now() >= exp) {
                return RespType::BulkString(None);
            }
            let val = match &entry.value {
                Value::String(v) => RespType::BulkString(Some(v.clone())),
                _ => return wrong_type(),
            };
            match expiry {
                Some(d) if d.is_zero() => entry.expiry = None,
                Some(d) => entry.expiry = Some(Instant::now() + d),
                None => {}
            }
            entry.version = bump_version();
            val
        }
        None => RespType::BulkString(None),
    })
}

pub fn handle_getdel(key: &str) -> RespType {
    with_db(|db| match db.remove(key) {
        Some(entry) => {
            if entry.expiry.is_some_and(|exp| Instant::now() >= exp) {
                return RespType::BulkString(None);
            }
            match entry.value {
                Value::String(v) => RespType::BulkString(Some(v)),
                _ => wrong_type(),
            }
        }
        None => RespType::BulkString(None),
    })
}

// ── BITFIELD helpers ──

fn parse_bf_encoding(enc: &str) -> Result<(bool, u32), ()> {
    let (sign, bits) = if let Some(bits_str) = enc.strip_prefix('i') {
        (true, bits_str)
    } else if let Some(bits_str) = enc.strip_prefix('u') {
        (false, bits_str)
    } else {
        return Err(());
    };
    let bit_count: u32 = bits.parse().map_err(|_| ())?;
    if !(1..=64).contains(&bit_count) { return Err(()); }
    Ok((sign, bit_count))
}

fn bf_offset(offset: i64, bit_count: u32) -> usize {
    if offset >= 0 {
        (offset as usize) * (bit_count as usize)
    } else {
        offset as usize
    }
}

fn bf_get_bits(data: &[u8], bit_offset: usize, bit_count: u32, signed: bool) -> i64 {
    let mut val: u64 = 0;
    for i in 0..bit_count as usize {
        let byte_idx = (bit_offset + i) / 8;
        let bit_idx = 7 - ((bit_offset + i) % 8);
        if byte_idx < data.len() && (data[byte_idx] >> bit_idx) & 1 != 0 {
            val |= 1u64 << (bit_count as usize - 1 - i);
        }
    }
    if signed && (val & (1u64 << (bit_count - 1))) != 0 {
        let mask = (1u64 << bit_count) - 1;
        (val | !mask) as i64
    } else {
        val as i64
    }
}

fn bf_set_bits(data: &mut Vec<u8>, bit_offset: usize, bit_count: u32, value: i64, _signed: bool) -> i64 {
    let old = bf_get_bits(data, bit_offset, bit_count, _signed);
    let end_bit = bit_offset + bit_count as usize;
    if end_bit > data.len() * 8 {
        data.resize((end_bit + 7) / 8, 0);
    }
    for i in 0..bit_count as usize {
        let byte_idx = (bit_offset + i) / 8;
        let bit_idx = 7 - ((bit_offset + i) % 8);
        if ((value as u64) >> (bit_count as usize - 1 - i)) & 1 != 0 {
            data[byte_idx] |= 1 << bit_idx;
        } else {
            data[byte_idx] &= !(1 << bit_idx);
        }
    }
    old
}

fn bf_wrap(value: i64, signed: bool, bit_count: u32) -> i64 {
    if signed {
        let range = 1i64 << (bit_count - 1);
        let mask = (1i64 << bit_count) - 1;
        let wrapped = value.wrapping_add(range) & mask;
        wrapped.wrapping_sub(range)
    } else {
        let mask = (1u64 << bit_count) - 1;
        (value as u64 & mask) as i64
    }
}

fn bf_clamp(value: i64, signed: bool, bit_count: u32) -> i64 {
    if signed {
        let max = (1i64 << (bit_count - 1)) - 1;
        let min = -(1i64 << (bit_count - 1));
        value.clamp(min, max)
    } else {
        let max = (1u64 << bit_count) - 1;
        (value as u64).clamp(0, max) as i64
    }
}

fn bf_incrby(
    data: &mut Vec<u8>, bit_offset: usize, bit_count: u32,
    increment: i64, signed: bool, overflow: &str,
) -> Option<i64> {
    let old = bf_get_bits(data, bit_offset, bit_count, signed);
    let new_val = old.wrapping_add(increment);
    match overflow {
        "SAT" => {
            let clamped = bf_clamp(new_val, signed, bit_count);
            bf_set_bits(data, bit_offset, bit_count, clamped, signed);
            Some(bf_get_bits(data, bit_offset, bit_count, signed))
        }
        "FAIL" => {
            let clamped = bf_clamp(new_val, signed, bit_count);
            if clamped != new_val {
                return None;
            }
            bf_set_bits(data, bit_offset, bit_count, new_val, signed);
            Some(new_val)
        }
        _ => {
            let wrapped = bf_wrap(new_val, signed, bit_count);
            bf_set_bits(data, bit_offset, bit_count, wrapped, signed);
            Some(bf_get_bits(data, bit_offset, bit_count, signed))
        }
    }
}

pub fn handle_bitfield(key: &str, sub_commands: &[BitFieldSub]) -> RespType {
    let results = with_db(|db| {
        let entry = db.entry(key.to_string()).or_insert_with(|| {
            Entry::new(Value::String(Bytes::new()), None)
        });
        match &mut entry.value {
            Value::String(v) => {
                let mut data = v.to_vec();
                let mut overflow = "WRAP";
                let mut res = Vec::new();
                for sub in sub_commands {
                    match sub {
                        BitFieldSub::Get { encoding, offset } => {
                            let (signed, bits) = parse_bf_encoding(encoding).unwrap_or((false, 8));
                            let bit_off = bf_offset(*offset, bits);
                            res.push(RespType::Integer(bf_get_bits(&data, bit_off, bits, signed)));
                        }
                        BitFieldSub::Set { encoding, offset, value } => {
                            let (signed, bits) = parse_bf_encoding(encoding).unwrap_or((false, 8));
                            let bit_off = bf_offset(*offset, bits);
                            let old = bf_set_bits(&mut data, bit_off, bits, *value, signed);
                            res.push(RespType::Integer(old));
                        }
                        BitFieldSub::Incrby { encoding, offset, increment } => {
                            let (signed, bits) = parse_bf_encoding(encoding).unwrap_or((false, 8));
                            let bit_off = bf_offset(*offset, bits);
                            match bf_incrby(&mut data, bit_off, bits, *increment, signed, overflow) {
                                Some(val) => res.push(RespType::Integer(val)),
                                None => res.push(RespType::BulkString(None)),
                            }
                        }
                        BitFieldSub::Overflow { behavior } => overflow = behavior,
                    }
                }
                if !data.is_empty() { *v = Bytes::from(data); }
                entry.version = bump_version();
                res
            }
            _ => vec![RespType::Error("WRONGTYPE Operation against a key holding the wrong kind of value".to_string())],
        }
    });
    RespType::Array(Some(results))
}

pub fn handle_bitfield_ro(key: &str, sub_commands: &[BitFieldSub]) -> RespType {
    let results = with_db(|db| match db.get(key) {
        Some(entry) => {
            if entry.expiry.is_some_and(|exp| Instant::now() >= exp) {
                return sub_commands.iter().map(|_| RespType::Integer(0)).collect();
            }
            match &entry.value {
                Value::String(v) => {
                    let data = v.to_vec();
                    let mut res = Vec::new();
                    for sub in sub_commands {
                        match sub {
                            BitFieldSub::Get { encoding, offset } => {
                                let (signed, bits) = parse_bf_encoding(encoding).unwrap_or((false, 8));
                                let bit_off = bf_offset(*offset, bits);
                                res.push(RespType::Integer(bf_get_bits(&data, bit_off, bits, signed)));
                            }
                            _ => res.push(RespType::Error("ERR BITFIELD_RO only supports GET".to_string())),
                        }
                    }
                    res
                }
                _ => vec![RespType::Error("WRONGTYPE".to_string())],
            }
        }
        None => sub_commands.iter().map(|_| RespType::Integer(0)).collect(),
    });
    RespType::Array(Some(results))
}

fn wrong_type() -> RespType {
    resp::RespType::Error(
        "WRONGTYPE Operation against a key holding the wrong kind of value".to_string(),
    )
}
