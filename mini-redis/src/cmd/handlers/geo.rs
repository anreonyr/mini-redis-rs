use bytes::Bytes;
use std::collections::BTreeSet;

use crate::storage::db::{with_db, Entry, Value};
use crate::protocol::resp::RespType;

const EARTH_RADIUS_M: f64 = 6372797.560856;

fn encode_geohash(lat: f64, lon: f64) -> i64 {
    let lat_offset = ((lat + 90.0) / 180.0 * ((1u64 << 26) as f64)) as u64;
    let lon_offset = ((lon + 180.0) / 360.0 * ((1u64 << 26) as f64)) as u64;
    let mut result: u64 = 0;
    for i in 0..26 {
        result |= ((lon_offset >> i) & 1) << (2 * i + 1);
        result |= ((lat_offset >> i) & 1) << (2 * i);
    }
    result as i64
}

fn decode_geohash(encoded: i64) -> (f64, f64) {
    let val = encoded as u64;
    let mut lat_offset: u64 = 0;
    let mut lon_offset: u64 = 0;
    for i in 0..26 {
        lat_offset |= (val >> (2 * i) & 1) << i;
        lon_offset |= (val >> (2 * i + 1) & 1) << i;
    }
    let lat = (lat_offset as f64 / (1u64 << 26) as f64) * 180.0 - 90.0;
    let lon = (lon_offset as f64 / (1u64 << 26) as f64) * 360.0 - 180.0;
    (lat, lon)
}

fn haversine(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let d_lat = (lat2 - lat1).to_radians();
    let d_lon = (lon2 - lon1).to_radians();
    let a = (d_lat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (d_lon / 2.0).sin().powi(2);
    EARTH_RADIUS_M * 2.0 * a.sqrt().asin()
}

fn convert_dist(meters: f64, unit: &str) -> f64 {
    match unit {
        "km" => meters / 1000.0,
        "mi" => meters / 1609.344,
        "ft" => meters * 3.28084,
        _ => meters,
    }
}

fn parse_unit_to_meters(radius: f64, unit: &str) -> f64 {
    match unit {
        "km" => radius * 1000.0,
        "mi" => radius * 1609.344,
        "ft" => radius / 3.28084,
        _ => radius,
    }
}

fn get_member_pos(zset: &BTreeSet<(i64, Bytes)>, member: &[u8]) -> Option<(f64, f64)> {
    for (score, m) in zset.iter() {
        if m.as_ref() == member {
            return Some(decode_geohash(*score));
        }
    }
    None
}

fn wrong_type() -> RespType {
    RespType::Error("WRONGTYPE Operation against a key holding the wrong kind of value".to_string())
}

pub fn handle_geoadd(key: &str, members: &[(f64, f64, String)]) -> RespType {
    with_db(|db| {
        let entry = db.entry(key.to_string()).or_insert_with(|| {
            Entry::new(Value::ZSet(BTreeSet::new()), None)
        });
        match &mut entry.value {
            Value::ZSet(zset) => {
                let mut added = 0i64;
                for (lon, lat, member) in members {
                    let geohash = encode_geohash(*lat, *lon);
                    let member_bytes = Bytes::copy_from_slice(member.as_bytes());
                    // Remove old entry if exists
                    zset.retain(|(_, m)| m.as_ref() != member.as_bytes());
                    zset.insert((geohash, member_bytes));
                    added += 1;
                }
                entry.version = crate::storage::db::bump_version();
                RespType::Integer(added)
            }
            _ => wrong_type(),
        }
    })
}

pub fn handle_geodist(key: &str, member1: &str, member2: &str, unit: &str) -> RespType {
    with_db(|db| {
        let entry = match db.get(key) {
            Some(e) => e,
            None => return RespType::BulkString(None),
        };
        let zset = match &entry.value {
            Value::ZSet(z) => z,
            _ => return RespType::Error("WRONGTYPE Operation against a key holding the wrong kind of value".to_string()),
        };
        let pos1 = get_member_pos(zset, member1.as_bytes());
        let pos2 = get_member_pos(zset, member2.as_bytes());
        match (pos1, pos2) {
            (Some((lat1, lon1)), Some((lat2, lon2))) => {
                let dist = convert_dist(haversine(lat1, lon1, lat2, lon2), unit);
                let s = if dist.fract() == 0.0 {
                    format!("{}.0", dist as i64)
                } else {
                    format!("{:.4}", dist)
                };
                RespType::BulkString(Some(Bytes::copy_from_slice(s.as_bytes())))
            }
            _ => RespType::BulkString(None),
        }
    })
}

pub fn handle_geohash(key: &str, members: &[String]) -> RespType {
    with_db(|db| {
        let entry = match db.get(key) {
            Some(e) => e,
            None => return RespType::Array(Some(vec![RespType::BulkString(None); members.len()])),
        };
        let zset = match &entry.value {
            Value::ZSet(z) => z,
            _ => return RespType::Error("WRONGTYPE Operation against a key holding the wrong kind of value".to_string()),
        };
        let results: Vec<RespType> = members.iter().map(|member| {
            for (score, m) in zset.iter() {
                if m.as_ref() == member.as_bytes() {
                    let hash_str = format!("{:x}", *score as u64);
                    return RespType::BulkString(Some(Bytes::copy_from_slice(hash_str.as_bytes())));
                }
            }
            RespType::BulkString(None)
        }).collect();
        RespType::Array(Some(results))
    })
}

pub fn handle_geopos(key: &str, members: &[String]) -> RespType {
    with_db(|db| {
        let entry = match db.get(key) {
            Some(e) => e,
            None => return RespType::Array(Some(vec![RespType::Array(None); members.len()])),
        };
        let zset = match &entry.value {
            Value::ZSet(z) => z,
            _ => return RespType::Error("WRONGTYPE Operation against a key holding the wrong kind of value".to_string()),
        };
        let results: Vec<RespType> = members.iter().map(|member| {
            for (score, m) in zset.iter() {
                if m.as_ref() == member.as_bytes() {
                    let (lat, lon) = decode_geohash(*score);
                    return RespType::Array(Some(vec![
                        RespType::BulkString(Some(Bytes::copy_from_slice(format!("{:.6}", lon).as_bytes()))),
                        RespType::BulkString(Some(Bytes::copy_from_slice(format!("{:.6}", lat).as_bytes()))),
                    ]));
                }
            }
            RespType::Array(None)
        }).collect();
        RespType::Array(Some(results))
    })
}

pub fn handle_georadius(
    key: &str, lon: f64, lat: f64, radius: f64, unit: &str,
    withcoord: bool, withdist: bool, count: Option<u64>,
) -> RespType {
    with_db(|db| {
        let entry = match db.get(key) {
            Some(e) => e,
            None => return RespType::Array(Some(vec![])),
        };
        let zset = match &entry.value {
            Value::ZSet(z) => z,
            _ => return RespType::Error("WRONGTYPE Operation against a key holding the wrong kind of value".to_string()),
        };
        let radius_m = parse_unit_to_meters(radius, unit);

        let mut results: Vec<(f64, &[u8], Option<(f64, f64)>)> = Vec::new();
        for (score, member) in zset.iter() {
            let (mlat, mlon) = decode_geohash(*score);
            let dist = haversine(lat, lon, mlat, mlon);
            if dist <= radius_m {
                results.push((dist, member.as_ref(), Some((mlat, mlon))));
            }
        }
        results.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        let max = count.unwrap_or(u64::MAX) as usize;
        let resp_results: Vec<RespType> = results.into_iter().take(max).map(|(dist, member, coord)| {
            let mut parts = Vec::new();
            parts.push(RespType::BulkString(Some(Bytes::copy_from_slice(member))));
            if withdist {
                let d = convert_dist(dist, unit);
                let s = if d.fract() == 0.0 {
                    format!("{}.0", d as i64)
                } else {
                    format!("{:.4}", d)
                };
                parts.push(RespType::BulkString(Some(Bytes::copy_from_slice(s.as_bytes()))));
            }
            if withcoord {
                if let Some((mlat, mlon)) = coord {
                    parts.push(RespType::Array(Some(vec![
                        RespType::BulkString(Some(Bytes::copy_from_slice(format!("{:.6}", mlon).as_bytes()))),
                        RespType::BulkString(Some(Bytes::copy_from_slice(format!("{:.6}", mlat).as_bytes()))),
                    ])));
                }
            }
            if parts.len() == 1 {
                parts.into_iter().next().unwrap()
            } else {
                RespType::Array(Some(parts))
            }
        }).collect();

        RespType::Array(Some(resp_results))
    })
}

pub fn handle_georadiusbymember(
    key: &str, member: &str, radius: f64, unit: &str,
    withcoord: bool, withdist: bool, count: Option<u64>,
) -> RespType {
    with_db(|db| {
        let entry = match db.get(key) {
            Some(e) => e,
            None => return RespType::Array(Some(vec![])),
        };
        let zset = match &entry.value {
            Value::ZSet(z) => z,
            _ => return RespType::Error("WRONGTYPE Operation against a key holding the wrong kind of value".to_string()),
        };
        let center = match get_member_pos(zset, member.as_bytes()) {
            Some(p) => p,
            None => return RespType::Array(Some(vec![])),
        };
        let (lat, lon) = center;
        let radius_m = parse_unit_to_meters(radius, unit);

        let mut results: Vec<(f64, &[u8], Option<(f64, f64)>)> = Vec::new();
        for (score, m) in zset.iter() {
            if m.as_ref() == member.as_bytes() { continue; }
            let (mlat, mlon) = decode_geohash(*score);
            let dist = haversine(lat, lon, mlat, mlon);
            if dist <= radius_m {
                results.push((dist, m.as_ref(), Some((mlat, mlon))));
            }
        }
        results.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        let max = count.unwrap_or(u64::MAX) as usize;
        let resp_results: Vec<RespType> = results.into_iter().take(max).map(|(dist, m, coord)| {
            let mut parts = Vec::new();
            parts.push(RespType::BulkString(Some(Bytes::copy_from_slice(m))));
            if withdist {
                let d = convert_dist(dist, unit);
                let s = if d.fract() == 0.0 {
                    format!("{}.0", d as i64)
                } else {
                    format!("{:.4}", d)
                };
                parts.push(RespType::BulkString(Some(Bytes::copy_from_slice(s.as_bytes()))));
            }
            if withcoord {
                if let Some((mlat, mlon)) = coord {
                    parts.push(RespType::Array(Some(vec![
                        RespType::BulkString(Some(Bytes::copy_from_slice(format!("{:.6}", mlon).as_bytes()))),
                        RespType::BulkString(Some(Bytes::copy_from_slice(format!("{:.6}", mlat).as_bytes()))),
                    ])));
                }
            }
            if parts.len() == 1 {
                parts.into_iter().next().unwrap()
            } else {
                RespType::Array(Some(parts))
            }
        }).collect();

        RespType::Array(Some(resp_results))
    })
}
