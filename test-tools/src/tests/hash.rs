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
        RespType::Array(Some(items)) => {
            let strs: Vec<String> = items.iter().map(|v| v.to_string()).collect();
            // Order: a, 1, b, 2 (or b, 2, a, 1)
            if strs.len() == 4 && strs[0] == "\"a\"" && strs[1] == "\"1\"" && strs[2] == "\"b\"" && strs[3] == "\"2\""
                || strs[0] == "\"b\"" && strs[1] == "\"2\"" && strs[2] == "\"a\"" && strs[3] == "\"1\""
            {
                Ok(())
            } else {
                Err(format!("HGETALL: unexpected order: {}", r))
            }
        }
        _ => Err(format!("HGETALL: expected Array, got {}", r)),
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
        RespType::Array(Some(items)) => {
            let strs: Vec<String> = items.iter().map(|v| v.to_string()).collect();
            if strs.len() == 2 && ((strs[0] == "\"name\"" && strs[1] == "\"age\"") || (strs[0] == "\"age\"" && strs[1] == "\"name\"")) {
                Ok(())
            } else {
                Err(format!("HKEYS: unexpected order: {}", r))
            }
        }
        _ => Err(format!("HKEYS: expected Array, got {}", r)),
    }
}

pub async fn test_hvals(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["HSET", "test_rs:h11", "name", "bob", "age", "30"]).await?;
    let r = client.cmd(&["HVALS", "test_rs:h11"]).await?;
    match &r {
        RespType::Array(Some(items)) => {
            let strs: Vec<String> = items.iter().map(|v| v.to_string()).collect();
            if strs.len() == 2 && ((strs[0] == "\"bob\"" && strs[1] == "\"30\"") || (strs[0] == "\"30\"" && strs[1] == "\"bob\"")) {
                Ok(())
            } else {
                Err(format!("HVALS: unexpected order: {}", r))
            }
        }
        _ => Err(format!("HVALS: expected Array, got {}", r)),
    }
}
