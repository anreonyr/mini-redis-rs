use crate::helpers::*;
use crate::RedisClient;
use mini_redis::resp::RespType;

pub async fn test_zadd_new_key(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["ZADD", "test_rs:z", "1", "a", "2", "b"]).await?;
    crate::assert_resp!(r, int(2), "ZADD new key");
    Ok(())
}

pub async fn test_zadd_update_score(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["ZADD", "test_rs:z2", "1", "a"]).await?;
    let r = client.cmd(&["ZADD", "test_rs:z2", "2", "a"]).await?;
    crate::assert_resp!(r, int(0), "ZADD update existing member returns 0");
    Ok(())
}

pub async fn test_zadd_existing_and_new(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["ZADD", "test_rs:z3", "1", "a"]).await?;
    let r = client.cmd(&["ZADD", "test_rs:z3", "2", "a", "3", "b"]).await?;
    // a exists (update), b is new
    crate::assert_resp!(r, int(1), "ZADD mixed existing/new");
    Ok(())
}

pub async fn test_zrange_by_index(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["ZADD", "test_rs:z4", "1", "a", "2", "b", "3", "c"]).await?;
    let r = client.cmd(&["ZRANGE", "test_rs:z4", "0", "-1"]).await?;
    crate::assert_resp!(r, arr_of_bulks(&["a", "b", "c"]), "ZRANGE full");
    Ok(())
}

pub async fn test_zrange_partial(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["ZADD", "test_rs:z5", "1", "a", "2", "b", "3", "c"]).await?;
    let r = client.cmd(&["ZRANGE", "test_rs:z5", "0", "1"]).await?;
    crate::assert_resp!(r, arr_of_bulks(&["a", "b"]), "ZRANGE partial");
    Ok(())
}

pub async fn test_zrange_withscores(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["ZADD", "test_rs:z6", "1", "a", "2", "b"]).await?;
    let r = client.cmd(&["ZRANGE", "test_rs:z6", "0", "-1", "WITHSCORES"]).await?;
    match &r {
        RespType::Array(Some(items)) => {
            if items.len() == 4 {
                Ok(())
            } else {
                Err(format!("ZRANGE WITHSCORES: expected 4 items, got {}", items.len()))
            }
        }
        _ => Err(format!("ZRANGE WITHSCORES: expected Array, got {}", r)),
    }
}

pub async fn test_zrange_empty_key(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["ZRANGE", "test_rs:nokey", "0", "-1"]).await?;
    crate::assert_resp!(r, empty_array(), "ZRANGE empty key");
    Ok(())
}

pub async fn test_zrank_existing(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["ZADD", "test_rs:z7", "10", "x", "20", "y", "30", "z"]).await?;
    let r = client.cmd(&["ZRANK", "test_rs:z7", "x"]).await?;
    crate::assert_resp!(r, int(0), "ZRANK lowest score");
    let r = client.cmd(&["ZRANK", "test_rs:z7", "z"]).await?;
    crate::assert_resp!(r, int(2), "ZRANK highest score");
    Ok(())
}

pub async fn test_zrank_nonexistent(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["ZADD", "test_rs:z8", "1", "a"]).await?;
    let r = client.cmd(&["ZRANK", "test_rs:z8", "b"]).await?;
    crate::assert_resp!(r, null_bulk(), "ZRANK nonexistent member");
    Ok(())
}

pub async fn test_zscore_existing(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["ZADD", "test_rs:z9", "42", "alice"]).await?;
    let r = client.cmd(&["ZSCORE", "test_rs:z9", "alice"]).await?;
    match &r {
        RespType::BulkString(Some(data)) => {
            let score_str = String::from_utf8_lossy(data);
            let score: i64 = score_str.parse().unwrap_or(0);
            if score == 42 {
                Ok(())
            } else {
                Err(format!("ZSCORE: expected 42, got {}", score_str))
            }
        }
        _ => Err(format!("ZSCORE: expected BulkString, got {}", r)),
    }
}

pub async fn test_zscore_nonexistent(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["ZSCORE", "test_rs:nokey", "x"]).await?;
    crate::assert_resp!(r, null_bulk(), "ZSCORE nonexistent key");
    Ok(())
}
