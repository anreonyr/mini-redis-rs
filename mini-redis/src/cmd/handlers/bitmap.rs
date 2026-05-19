use bytes::Bytes;

use crate::storage::db::{Entry, Value, with_db};
use crate::protocol::resp::RespType;

fn wrong_type() -> RespType {
    RespType::Error("WRONGTYPE Operation against a key holding the wrong kind of value".to_string())
}

/// Read bytes from a key (treating missing key as empty).
fn get_bytes(db: &mut std::collections::HashMap<String, Entry>, key: &str) -> Vec<u8> {
    match db.get(key) {
        Some(entry) => match &entry.value {
            Value::String(bytes) => bytes.to_vec(),
            _ => return Vec::new(), // caller handles wrong type separately
        },
        None => Vec::new(),
    }
}

/// Resize a byte vector to at least `min_len + 1` bytes, zero-filling.
fn resize_bytes(v: &mut Vec<u8>, min_len: usize) {
    if v.len() <= min_len {
        v.resize(min_len + 1, 0);
    }
}

/// Precomputed popcount for each byte value (const eval).
const POPCOUNT: [u8; 256] = {
    let mut table = [0u8; 256];
    let mut i = 0;
    while i < 256 {
        table[i] = (i as u8).count_ones() as u8;
        i += 1;
    }
    table
};

pub fn handle_getbit(key: &str, offset: u64) -> RespType {
    with_db(|db| {
        // Check wrong type
        if let Some(entry) = db.get(key) {
            if !matches!(entry.value, Value::String(_)) {
                return wrong_type();
            }
        }
        let bytes = get_bytes(db, key);
        let byte_idx = (offset / 8) as usize;
        let bit_idx = 7 - (offset % 8) as usize; // big-endian bit order
        if byte_idx >= bytes.len() {
            return RespType::Integer(0);
        }
        let bit = (bytes[byte_idx] >> bit_idx) & 1;
        RespType::Integer(bit as i64)
    })
}

pub fn handle_setbit(key: &str, offset: u64, value: u8) -> RespType {
    with_db(|db| {
        // Check wrong type
        if let Some(entry) = db.get(key) {
            if !matches!(entry.value, Value::String(_)) {
                return wrong_type();
            }
        }
        let entry = db.entry(key.to_string()).or_insert_with(|| {
            Entry::new(Value::String(Bytes::new()), None)
        });
        match &mut entry.value {
            Value::String(bytes) => {
                let byte_idx = (offset / 8) as usize;
                let bit_idx = 7 - (offset % 8) as usize;
                let mut v = bytes.to_vec();
                resize_bytes(&mut v, byte_idx);
                let old_bit = (v[byte_idx] >> bit_idx) & 1;
                if value == 1 {
                    v[byte_idx] |= 1 << bit_idx;
                } else {
                    v[byte_idx] &= !(1 << bit_idx);
                }
                *bytes = Bytes::from(v);
                entry.version = crate::storage::db::bump_version();
                RespType::Integer(old_bit as i64)
            }
            _ => wrong_type(),
        }
    })
}

pub fn handle_bitcount(key: &str, start: Option<i64>, end: Option<i64>) -> RespType {
    with_db(|db| {
        // Check wrong type
        if let Some(entry) = db.get(key) {
            if !matches!(entry.value, Value::String(_)) {
                return wrong_type();
            }
        }
        let bytes = get_bytes(db, key);
        if bytes.is_empty() {
            return RespType::Integer(0);
        }
        let len = bytes.len() as i64;
        let s = start.map(|s| if s < 0 { (len + s).max(0) } else { s }).unwrap_or(0).max(0) as usize;
        let e = end.map(|e| if e < 0 { (len + e).max(0) } else { e }).unwrap_or(len - 1).max(0) as usize;
        let e = e.min(bytes.len() - 1);
        let mut count: u64 = 0;
        for byte in &bytes[s..=e] {
            count += POPCOUNT[*byte as usize] as u64;
        }
        RespType::Integer(count as i64)
    })
}

pub fn handle_bitop(op: &str, dest: &str, keys: &[String]) -> RespType {
    with_db(|db| {
        // Check wrong type for all source keys
        for k in keys {
            if let Some(entry) = db.get(k) {
                if !matches!(entry.value, Value::String(_)) {
                    return wrong_type();
                }
            }
        }

        let sources: Vec<Vec<u8>> = keys.iter().map(|k| get_bytes(db, k)).collect();
        let max_len = sources.iter().map(|v| v.len()).max().unwrap_or(0);

        if max_len == 0 {
            // No data to operate on, remove destination
            db.remove(dest);
            return RespType::Integer(0);
        }

        let mut result = vec![0u8; max_len];

        match op {
            "AND" => {
                result.copy_from_slice(&sources[0]);
                for src in &sources[1..] {
                    for i in 0..max_len {
                        result[i] &= src.get(i).copied().unwrap_or(0);
                    }
                }
            }
            "OR" => {
                for src in &sources {
                    for i in 0..max_len {
                        result[i] |= src.get(i).copied().unwrap_or(0);
                    }
                }
            }
            "XOR" => {
                for src in &sources {
                    for i in 0..max_len {
                        result[i] ^= src.get(i).copied().unwrap_or(0);
                    }
                }
            }
            "NOT" => {
                if keys.len() != 1 {
                    return RespType::Error(
                        "ERR BITOP NOT must be called with a single source key".to_string(),
                    );
                }
                result.copy_from_slice(&sources[0]);
                for i in 0..max_len {
                    result[i] = !result[i];
                }
            }
            _ => return RespType::Error("ERR syntax error".to_string()),
        }

        let entry = db.entry(dest.to_string()).or_insert_with(|| {
            Entry::new(Value::String(Bytes::new()), None)
        });
        match &mut entry.value {
            Value::String(bytes) => {
                *bytes = Bytes::from(result);
                entry.version = crate::storage::db::bump_version();
                RespType::Integer(max_len as i64)
            }
            _ => wrong_type(),
        }
    })
}

pub fn handle_bitpos(key: &str, bit: u8, start: Option<i64>, end: Option<i64>) -> RespType {
    with_db(|db| {
        // Check wrong type
        if let Some(entry) = db.get(key) {
            if !matches!(entry.value, Value::String(_)) {
                return wrong_type();
            }
        }
        let bytes = get_bytes(db, key);
        if bytes.is_empty() {
            return if bit == 0 {
                RespType::Integer(0)
            } else {
                RespType::Integer(-1)
            };
        }
        let len = bytes.len() as i64;
        let s = start
            .map(|s| if s < 0 { (len + s).max(0) } else { s })
            .unwrap_or(0)
            .max(0) as usize;
        let e = end
            .map(|e| if e < 0 { (len + e).max(0) } else { e })
            .unwrap_or(len - 1)
            .max(0) as usize;
        let e = e.min(bytes.len() - 1);
        let search_bit = bit != 0;

        for byte_idx in s..=e {
            let byte = bytes[byte_idx];
            if search_bit && byte != 0 {
                // Find first set bit in this byte (big-endian order)
                for bit_idx in (0..8).rev() {
                    if (byte >> bit_idx) & 1 == 1 {
                        return RespType::Integer((byte_idx * 8 + (7 - bit_idx)) as i64);
                    }
                }
            } else if !search_bit && byte != 0xFF {
                // Find first clear bit in this byte (big-endian order)
                for bit_idx in (0..8).rev() {
                    if (byte >> bit_idx) & 1 == 0 {
                        return RespType::Integer((byte_idx * 8 + (7 - bit_idx)) as i64);
                    }
                }
            }
        }

        // Bit not found in range
        if bit == 0 && end.is_none() {
            // 0 not found within existing data - return first bit past the end
            return RespType::Integer((bytes.len() * 8) as i64);
        }
        RespType::Integer(-1)
    })
}
