use crate::helpers::*;
use crate::RedisClient;
use mini_redis::resp::RespType;

pub async fn test_xadd_basic(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["XADD", "test_rs:s1", "*", "field1", "value1"]).await?;
    match &r {
        RespType::BulkString(Some(id)) => {
            let id_str = String::from_utf8_lossy(id);
            if !id_str.contains('-') {
                return Err(format!("XADD: expected ID containing '-', got {}", id_str));
            }
            Ok(())
        }
        _ => Err(format!("XADD: expected BulkString ID, got {}", r)),
    }
}

pub async fn test_xadd_explicit_id(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["XADD", "test_rs:s_explicit", "1000-0", "f", "v"]).await?;
    crate::assert_resp!(r, bulk_str("1000-0"), "XADD explicit ID");
    let r2 = client.cmd(&["XLEN", "test_rs:s_explicit"]).await?;
    crate::assert_resp!(r2, int(1), "XLEN after explicit XADD");
    Ok(())
}

pub async fn test_xadd_sequence_auto(client: &mut RedisClient) -> Result<(), String> {
    let r1 = client.cmd(&["XADD", "test_rs:s_auto", "100-0", "f", "v"]).await?;
    crate::assert_resp!(r1, bulk_str("100-0"), "XADD 100-0");
    let r2 = client.cmd(&["XADD", "test_rs:s_auto", "100-*", "f", "v2"]).await?;
    match &r2 {
        RespType::BulkString(Some(id)) => {
            let id_str = String::from_utf8_lossy(id);
            if id_str != "100-1" {
                return Err(format!("XADD 100-*: expected '100-1', got {}", id_str));
            }
            Ok(())
        }
        _ => Err(format!("XADD 100-*: expected BulkString, got {}", r2)),
    }
}

pub async fn test_xlen(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["XLEN", "test_rs:s1"]).await?;
    crate::assert_resp!(r, int(1), "XLEN existing stream");
    let r2 = client.cmd(&["XLEN", "test_rs:nox"]).await?;
    crate::assert_resp!(r2, int(0), "XLEN nonexistent stream");
    Ok(())
}

pub async fn test_xadd_multiple(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["XADD", "test_rs:s2", "0-1", "a", "1"]).await?;
    let _ = client.cmd(&["XADD", "test_rs:s2", "0-2", "b", "2"]).await?;
    let _ = client.cmd(&["XADD", "test_rs:s2", "0-3", "c", "3"]).await?;
    let r = client.cmd(&["XLEN", "test_rs:s2"]).await?;
    crate::assert_resp!(r, int(3), "XLEN after 3 XADDs");
    Ok(())
}

pub async fn test_xrange_full(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["XRANGE", "test_rs:s2", "-", "+"]).await?;
    match &r {
        RespType::Array(Some(entries)) => {
            if entries.len() != 3 {
                return Err(format!("XRANGE full: expected 3 entries, got {}", entries.len()));
            }
            for (i, entry) in entries.iter().enumerate() {
                match entry {
                    RespType::Array(Some(parts)) if parts.len() == 2 => {}
                    _ => return Err(format!(
                        "XRANGE entry {}: expected Array[2], got {}", i, entry)),
                }
            }
            Ok(())
        }
        _ => Err(format!("XRANGE full: expected Array, got {}", r)),
    }
}

pub async fn test_xrange_range(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["XRANGE", "test_rs:s2", "0-2", "0-3"]).await?;
    match &r {
        RespType::Array(Some(entries)) if entries.len() == 2 => Ok(()),
        RespType::Array(Some(entries)) => Err(format!(
            "XRANGE 0-2 0-3: expected 2 entries, got {}", entries.len())),
        _ => Err(format!("XRANGE range: expected Array, got {}", r)),
    }
}

pub async fn test_xrange_count(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["XRANGE", "test_rs:s2", "-", "+", "COUNT", "2"]).await?;
    match &r {
        RespType::Array(Some(entries)) if entries.len() == 2 => Ok(()),
        RespType::Array(Some(entries)) => Err(format!(
            "XRANGE COUNT 2: expected 2 entries, got {}", entries.len())),
        _ => Err(format!("XRANGE COUNT: expected Array, got {}", r)),
    }
}

pub async fn test_xrevrange(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["XREVRANGE", "test_rs:s2", "+", "-"]).await?;
    match &r {
        RespType::Array(Some(entries)) => {
            if entries.len() != 3 {
                return Err(format!("XREVRANGE: expected 3 entries, got {}", entries.len()));
            }
            Ok(())
        }
        _ => Err(format!("XREVRANGE: expected Array, got {}", r)),
    }
}

pub async fn test_xtrim(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["XTRIM", "test_rs:s2", "MAXLEN", "2"]).await?;
    crate::assert_resp!(r, int(1), "XTRIM removed 1");
    let r2 = client.cmd(&["XLEN", "test_rs:s2"]).await?;
    crate::assert_resp!(r2, int(2), "XLEN after XTRIM");
    Ok(())
}

pub async fn test_xdel(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["XDEL", "test_rs:s2", "0-2"]).await?;
    crate::assert_resp!(r, int(1), "XDEL 0-2");
    let r2 = client.cmd(&["XLEN", "test_rs:s2"]).await?;
    crate::assert_resp!(r2, int(1), "XLEN after XDEL");
    Ok(())
}

pub async fn test_xread_basic(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["XREAD", "STREAMS", "test_rs:s2", "0"]).await?;
    match &r {
        RespType::Array(Some(streams)) => {
            if streams.is_empty() {
                return Err("XREAD: expected non-empty array".to_string());
            }
            for (i, se) in streams.iter().enumerate() {
                match se {
                    RespType::Array(Some(parts)) if parts.len() == 2 => {}
                    _ => return Err(format!(
                        "XREAD stream {}: expected Array[2], got {}", i, se)),
                }
            }
            Ok(())
        }
        _ => Err(format!("XREAD: expected Array, got {}", r)),
    }
}

pub async fn test_xread_count(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["XREAD", "COUNT", "1", "STREAMS", "test_rs:s2", "0"]).await?;
    match &r {
        RespType::Array(Some(streams)) => {
            if streams.is_empty() {
                return Err("XREAD COUNT: expected non-empty array".to_string());
            }
            Ok(())
        }
        _ => Err(format!("XREAD COUNT: expected Array, got {}", r)),
    }
}

pub async fn test_xread_multi_key(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["XREAD", "STREAMS", "test_rs:s1", "test_rs:s2", "0", "0"]).await?;
    match &r {
        RespType::Array(Some(streams)) => {
            if streams.len() < 2 {
                return Err(format!(
                    "XREAD multi: expected >= 2 streams, got {}", streams.len()));
            }
            Ok(())
        }
        _ => Err(format!("XREAD multi: expected Array, got {}", r)),
    }
}

pub async fn test_wrongtype_xadd_on_string(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SET", "test_rs:stream_wt", "val"]).await?;
    let r = client.cmd(&["XADD", "test_rs:stream_wt", "*", "f", "v"]).await?;
    match &r {
        RespType::Error(msg) if msg.to_lowercase().contains("wrongtype") => Ok(()),
        _ => Err(format!("XADD on string: expected WRONGTYPE, got {}", r)),
    }
}
