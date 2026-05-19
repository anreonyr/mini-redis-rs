use crate::helpers::*;
use crate::RedisClient;
use mini_redis::resp::RespType;

pub async fn test_hset_new_key(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["HSET", "test_rs:h", "field1", "val1"]).await?;
    crate::assert_resp!(r, int(1), "HSET new key single field");
    Ok(())
}

pub async fn test_hset_multiple_fields(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["HSET", "test_rs:h", "f1", "v1", "f2", "v2", "f3", "v3"]).await?;
    crate::assert_resp!(r, int(3), "HSET multiple fields");
    Ok(())
}

pub async fn test_hset_overwrite(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["HSET", "test_rs:h2", "field", "old"]).await?;
    let r = client.cmd(&["HSET", "test_rs:h2", "field", "new"]).await?;
    crate::assert_resp!(r, int(0), "HSET overwrite existing field returns 0");
    Ok(())
}

pub async fn test_hget_existing(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["HSET", "test_rs:h3", "name", "alice"]).await?;
    let r = client.cmd(&["HGET", "test_rs:h3", "name"]).await?;
    crate::assert_resp!(r, bulk_str("alice"), "HGET existing field");
    Ok(())
}

pub async fn test_hget_nonexistent(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["HGET", "test_rs:nonexh", "nofield"]).await?;
    crate::assert_resp!(r, null_bulk(), "HGET nonexistent field");
    Ok(())
}

pub async fn test_hget_nonexistent_key(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["HGET", "test_rs:nokey", "field"]).await?;
    crate::assert_resp!(r, null_bulk(), "HGET nonexistent key");
    Ok(())
}

pub async fn test_hdel_single(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["HSET", "test_rs:h4", "a", "1", "b", "2"]).await?;
    let r = client.cmd(&["HDEL", "test_rs:h4", "a"]).await?;
    crate::assert_resp!(r, int(1), "HDEL single field");
    Ok(())
}

pub async fn test_hdel_multiple(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["HSET", "test_rs:h5", "x", "10", "y", "20", "z", "30"]).await?;
    let r = client.cmd(&["HDEL", "test_rs:h5", "x", "y"]).await?;
    crate::assert_resp!(r, int(2), "HDEL multiple fields");
    Ok(())
}

pub async fn test_hdel_nonexistent(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["HDEL", "test_rs:nonexhdel", "field"]).await?;
    crate::assert_resp!(r, int(0), "HDEL nonexistent key");
    Ok(())
}

pub async fn test_hgetall_full(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["HSET", "test_rs:h6", "a", "1", "b", "2"]).await?;
    let r = client.cmd(&["HGETALL", "test_rs:h6"]).await?;
    match &r {
        RespType::Array(Some(items)) if items.len() == 4 => {
            let a = bulk_str("a");
            let b1 = bulk_str("1");
            let c = bulk_str("b");
            let d = bulk_str("2");
            let order1 = &items[0] == &a && &items[1] == &b1 && &items[2] == &c && &items[3] == &d;
            let order2 = &items[0] == &c && &items[1] == &d && &items[2] == &a && &items[3] == &b1;
            if order1 || order2 {
                Ok(())
            } else {
                Err(format!("HGETALL: unexpected order: {}", r))
            }
        }
        _ => Err(format!("HGETALL: expected Array of 4 items, got {}", r)),
    }
}

pub async fn test_hgetall_empty(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["HGETALL", "test_rs:emptyh"]).await?;
    crate::assert_resp!(r, empty_array(), "HGETALL empty key");
    Ok(())
}

pub async fn test_hexists_true(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["HSET", "test_rs:h7", "field", "val"]).await?;
    let r = client.cmd(&["HEXISTS", "test_rs:h7", "field"]).await?;
    crate::assert_resp!(r, int(1), "HEXISTS existing field");
    Ok(())
}

pub async fn test_hexists_false(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["HSET", "test_rs:h8", "field", "val"]).await?;
    let r = client.cmd(&["HEXISTS", "test_rs:h8", "nope"]).await?;
    crate::assert_resp!(r, int(0), "HEXISTS nonexistent field");
    Ok(())
}

pub async fn test_hlen(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["HSET", "test_rs:h9", "a", "1", "b", "2", "c", "3"]).await?;
    let r = client.cmd(&["HLEN", "test_rs:h9"]).await?;
    crate::assert_resp!(r, int(3), "HLEN");
    Ok(())
}

pub async fn test_hlen_empty(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["HLEN", "test_rs:nohash"]).await?;
    crate::assert_resp!(r, int(0), "HLEN empty key");
    Ok(())
}

pub async fn test_hkeys(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["HSET", "test_rs:h10", "name", "bob", "age", "30"]).await?;
    let r = client.cmd(&["HKEYS", "test_rs:h10"]).await?;
    match &r {
        RespType::Array(Some(items)) if items.len() == 2 => {
            let name = bulk_str("name");
            let age = bulk_str("age");
            let order1 = &items[0] == &name && &items[1] == &age;
            let order2 = &items[0] == &age && &items[1] == &name;
            if order1 || order2 {
                Ok(())
            } else {
                Err(format!("HKEYS: unexpected order: {}", r))
            }
        }
        _ => Err(format!("HKEYS: expected Array of 2 items, got {}", r)),
    }
}

pub async fn test_hvals(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["HSET", "test_rs:h11", "name", "bob", "age", "30"]).await?;
    let r = client.cmd(&["HVALS", "test_rs:h11"]).await?;
    match &r {
        RespType::Array(Some(items)) if items.len() == 2 => {
            let bob = bulk_str("bob");
            let thirty = bulk_str("30");
            let order1 = &items[0] == &bob && &items[1] == &thirty;
            let order2 = &items[0] == &thirty && &items[1] == &bob;
            if order1 || order2 {
                Ok(())
            } else {
                Err(format!("HVALS: unexpected order: {}", r))
            }
        }
        _ => Err(format!("HVALS: expected Array of 2 items, got {}", r)),
    }
}

pub async fn test_hincrby(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["HSET", "test_rs:hi", "counter", "10"]).await?;
    let r = client.cmd(&["HINCRBY", "test_rs:hi", "counter", "5"]).await?;
    crate::assert_resp!(r, int(15), "HINCRBY");
    Ok(())
}

pub async fn test_hincrby_new(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["HINCRBY", "test_rs:hi2", "counter", "5"]).await?;
    crate::assert_resp!(r, int(5), "HINCRBY new field");
    Ok(())
}

pub async fn test_hincrbyfloat(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["HSET", "test_rs:hif", "val", "1.0"]).await?;
    let r = client.cmd(&["HINCRBYFLOAT", "test_rs:hif", "val", "0.5"]).await?;
    assert!(matches!(&r, RespType::BulkString(Some(_))), "HINCRBYFLOAT");
    Ok(())
}

pub async fn test_hsetnx(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["HSETNX", "test_rs:hsn", "field", "value"]).await?;
    crate::assert_resp!(r, int(1), "HSETNX new");
    let r = client.cmd(&["HSETNX", "test_rs:hsn", "field", "value2"]).await?;
    crate::assert_resp!(r, int(0), "HSETNX existing");
    Ok(())
}
