use crate::helpers::*;
use crate::RedisClient;
use mini_redis::resp::RespType;

pub async fn test_set_get_roundtrip(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["SET", "test_rs:val1", "value1"]).await?;
    crate::assert_resp!(r, simple_str("OK"), "SET basic");
    let r = client.cmd(&["GET", "test_rs:val1"]).await?;
    crate::assert_resp!(r, bulk_str("value1"), "GET basic");
    Ok(())
}

pub async fn test_get_nonexistent(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["GET", "test_rs:nonexist"]).await?;
    crate::assert_resp!(r, null_bulk(), "GET nonexistent");
    Ok(())
}

pub async fn test_set_overwrite(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SET", "test_rs:val1", "newval"]).await?;
    let r = client.cmd(&["GET", "test_rs:val1"]).await?;
    crate::assert_resp!(r, bulk_str("newval"), "SET overwrite");
    Ok(())
}

pub async fn test_set_with_ex(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["SET", "test_rs:exkey", "val", "EX", "7200"]).await?;
    crate::assert_resp!(r, simple_str("OK"), "SET EX");
    let r = client.cmd(&["GET", "test_rs:exkey"]).await?;
    crate::assert_resp!(r, bulk_str("val"), "GET after SET EX");
    Ok(())
}

pub async fn test_set_with_px(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["SET", "test_rs:pxkey", "val", "PX", "7200000"]).await?;
    crate::assert_resp!(r, simple_str("OK"), "SET PX");
    let r = client.cmd(&["GET", "test_rs:pxkey"]).await?;
    crate::assert_resp!(r, bulk_str("val"), "GET after SET PX");
    Ok(())
}

pub async fn test_set_wrong_args(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["SET", "key"]).await?;
    crate::assert_match!(r, RespType::Error(_), "SET wrong args");
    Ok(())
}

pub async fn test_set_invalid_flag(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["SET", "k", "v", "XX", "100"]).await?;
    match &r {
        RespType::Error(msg) if msg.to_lowercase().contains("syntax") => Ok(()),
        _ => Err(format!("SET invalid flag: expected syntax error, got {}", r)),
    }
}

pub async fn test_set_invalid_expiry(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["SET", "k", "v", "EX", "abc"]).await?;
    match &r {
        RespType::Error(msg) if msg.to_lowercase().contains("not an integer") => Ok(()),
        _ => Err(format!("SET invalid expiry: expected 'not an integer', got {}", r)),
    }
}

pub async fn test_set_empty_value(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["SET", "test_rs:empty", ""]).await?;
    crate::assert_resp!(r, simple_str("OK"), "SET empty value");
    let r = client.cmd(&["GET", "test_rs:empty"]).await?;
    crate::assert_resp!(r, bulk_str(""), "GET empty value");
    Ok(())
}

pub async fn test_set_binary_data(client: &mut RedisClient) -> Result<(), String> {
    let key = "test_rs:bin";
    let value = "value_with_null_\x00_and_ff_\u{ff}";
    let r = client.cmd(&["SET", key, value]).await?;
    crate::assert_resp!(r, simple_str("OK"), "SET binary");
    let r = client.cmd(&["GET", key]).await?;
    match &r {
        RespType::BulkString(Some(data)) if data[..] == value.as_bytes()[..] => Ok(()),
        RespType::BulkString(Some(data)) => Err(format!("GET binary: data mismatch, got {:?}", data)),
        _ => Err(format!("GET binary: expected BulkString, got {}", r)),
    }
}
