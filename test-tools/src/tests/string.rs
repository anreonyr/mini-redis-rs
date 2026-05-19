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

pub async fn test_incr_new_key(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["INCR", "counter"]).await?;
    crate::assert_resp!(r, int(1), "INCR new key");
    Ok(())
}

pub async fn test_incr_existing(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SET", "counter2", "10"]).await?;
    let r = client.cmd(&["INCR", "counter2"]).await?;
    crate::assert_resp!(r, int(11), "INCR existing");
    Ok(())
}

pub async fn test_decr(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SET", "counter3", "10"]).await?;
    let r = client.cmd(&["DECR", "counter3"]).await?;
    crate::assert_resp!(r, int(9), "DECR");
    Ok(())
}

pub async fn test_incrby(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SET", "c4", "10"]).await?;
    let r = client.cmd(&["INCRBY", "c4", "5"]).await?;
    crate::assert_resp!(r, int(15), "INCRBY");
    Ok(())
}

pub async fn test_decrby(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SET", "c5", "10"]).await?;
    let r = client.cmd(&["DECRBY", "c5", "3"]).await?;
    crate::assert_resp!(r, int(7), "DECRBY");
    Ok(())
}

pub async fn test_incr_wrong_type(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["RPUSH", "mylist", "a"]).await?;
    let r = client.cmd(&["INCR", "mylist"]).await?;
    crate::assert_match!(r, RespType::Error(_), "INCR on list should error");
    Ok(())
}

pub async fn test_incr_invalid_value(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SET", "notanum", "hello"]).await?;
    let r = client.cmd(&["INCR", "notanum"]).await?;
    crate::assert_match!(r, RespType::Error(_), "INCR on non-integer should error");
    Ok(())
}

pub async fn test_append(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SET", "ap", "hello"]).await?;
    let r = client.cmd(&["APPEND", "ap", " world"]).await?;
    crate::assert_resp!(r, int(11), "APPEND return length");
    let r = client.cmd(&["GET", "ap"]).await?;
    crate::assert_resp!(r, bulk_str("hello world"), "APPEND result");
    Ok(())
}

pub async fn test_append_new_key(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["APPEND", "ap2", "hello"]).await?;
    crate::assert_resp!(r, int(5), "APPEND new key");
    Ok(())
}

pub async fn test_strlen(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SET", "st", "hello"]).await?;
    let r = client.cmd(&["STRLEN", "st"]).await?;
    crate::assert_resp!(r, int(5), "STRLEN");
    Ok(())
}

pub async fn test_strlen_nonexistent(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["STRLEN", "no"]).await?;
    crate::assert_resp!(r, int(0), "STRLEN nonexistent");
    Ok(())
}

pub async fn test_mget(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SET", "m1", "v1"]).await?;
    let _ = client.cmd(&["SET", "m2", "v2"]).await?;
    let r = client.cmd(&["MGET", "m1", "m2", "m3"]).await?;
    assert!(matches!(&r, RespType::Array(Some(v)) if v.len() == 3), "MGET array of 3, got {}", r);
    Ok(())
}

pub async fn test_mset(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["MSET", "k1", "v1", "k2", "v2"]).await?;
    crate::assert_resp!(r, simple_str("OK"), "MSET");
    let r = client.cmd(&["GET", "k1"]).await?;
    crate::assert_resp!(r, bulk_str("v1"), "MSET k1");
    let r = client.cmd(&["GET", "k2"]).await?;
    crate::assert_resp!(r, bulk_str("v2"), "MSET k2");
    Ok(())
}

pub async fn test_getset(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SET", "gs", "old"]).await?;
    let r = client.cmd(&["GETSET", "gs", "new"]).await?;
    crate::assert_resp!(r, bulk_str("old"), "GETSET old value");
    let r = client.cmd(&["GET", "gs"]).await?;
    crate::assert_resp!(r, bulk_str("new"), "GETSET new value");
    Ok(())
}

pub async fn test_getrange(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SET", "gr", "hello"]).await?;
    let r = client.cmd(&["GETRANGE", "gr", "0", "2"]).await?;
    crate::assert_resp!(r, bulk_str("hel"), "GETRANGE 0-2");
    Ok(())
}

pub async fn test_setrange(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SET", "sr", "hello"]).await?;
    let r = client.cmd(&["SETRANGE", "sr", "1", "a"]).await?;
    crate::assert_resp!(r, int(5), "SETRANGE return length");
    let r = client.cmd(&["GET", "sr"]).await?;
    crate::assert_resp!(r, bulk_str("hallo"), "SETRANGE result");
    Ok(())
}

pub async fn test_msetnx(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["MSETNX", "n1", "v1", "n2", "v2"]).await?;
    crate::assert_resp!(r, int(1), "MSETNX should succeed");
    let r = client.cmd(&["MSETNX", "n1", "v3", "n3", "v3"]).await?;
    crate::assert_resp!(r, int(0), "MSETNX should fail when key exists");
    Ok(())
}
