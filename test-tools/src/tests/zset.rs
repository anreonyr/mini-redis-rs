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

pub async fn test_zrem_basic(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["ZADD", "test_rs:zr", "1", "a", "2", "b", "3", "c"]).await?;
    let r = client.cmd(&["ZREM", "test_rs:zr", "a", "c"]).await?;
    crate::assert_resp!(r, int(2), "ZREM 2 members");
    let r = client.cmd(&["ZCARD", "test_rs:zr"]).await?;
    crate::assert_resp!(r, int(1), "ZCARD after ZREM");
    Ok(())
}

pub async fn test_zrem_nonexistent(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["ZREM", "test_rs:nokey", "x"]).await?;
    crate::assert_resp!(r, int(0), "ZREM nonexistent key");
    Ok(())
}

pub async fn test_zcard(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["ZADD", "test_rs:zc", "1", "a", "2", "b", "3", "c"]).await?;
    let r = client.cmd(&["ZCARD", "test_rs:zc"]).await?;
    crate::assert_resp!(r, int(3), "ZCARD");
    Ok(())
}

pub async fn test_zcard_empty(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["ZCARD", "test_rs:nokey"]).await?;
    crate::assert_resp!(r, int(0), "ZCARD empty");
    Ok(())
}

pub async fn test_zcount(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["ZADD", "test_rs:zco", "1", "a", "2", "b", "3", "c", "4", "d"]).await?;
    let r = client.cmd(&["ZCOUNT", "test_rs:zco", "2", "4"]).await?;
    crate::assert_resp!(r, int(3), "ZCOUNT 2-4");
    Ok(())
}

pub async fn test_zcount_inf(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["ZCOUNT", "test_rs:zco", "-inf", "+inf"]).await?;
    crate::assert_resp!(r, int(4), "ZCOUNT -inf +inf");
    Ok(())
}

pub async fn test_zrangebyscore(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["ZADD", "test_rs:zrb", "1", "a", "2", "b", "3", "c", "4", "d"]).await?;
    let r = client.cmd(&["ZRANGEBYSCORE", "test_rs:zrb", "2", "4"]).await?;
    crate::assert_resp!(r, arr_of_bulks(&["b", "c", "d"]), "ZRANGEBYSCORE 2-4");
    Ok(())
}

pub async fn test_zrangebyscore_withscores(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["ZADD", "test_rs:zrbs", "1", "a", "2", "b"]).await?;
    let r = client.cmd(&["ZRANGEBYSCORE", "test_rs:zrbs", "1", "2", "WITHSCORES"]).await?;
    assert!(matches!(&r, RespType::Array(Some(v)) if v.len() == 4), "ZRANGEBYSCORE WITHSCORES");
    Ok(())
}

pub async fn test_zincrby(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["ZADD", "test_rs:zi", "10", "a"]).await?;
    let r = client.cmd(&["ZINCRBY", "test_rs:zi", "5", "a"]).await?;
    assert!(matches!(&r, RespType::BulkString(Some(_))), "ZINCRBY response");
    let r = client.cmd(&["ZSCORE", "test_rs:zi", "a"]).await?;
    match &r {
        RespType::BulkString(Some(data)) if String::from_utf8_lossy(data) == "15" => Ok(()),
        _ => Err(format!("ZINCRBY: expected score 15, got {}", r)),
    }
}

pub async fn test_zincrby_new(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["ZINCRBY", "test_rs:zi2", "5", "newmember"]).await?;
    assert!(matches!(&r, RespType::BulkString(Some(_))), "ZINCRBY new member");
    Ok(())
}

pub async fn test_zrevrange(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["ZADD", "test_rs:zrv", "1", "a", "2", "b", "3", "c"]).await?;
    let r = client.cmd(&["ZREVRANGE", "test_rs:zrv", "0", "-1"]).await?;
    crate::assert_resp!(r, arr_of_bulks(&["c", "b", "a"]), "ZREVRANGE full");
    Ok(())
}

pub async fn test_zrevrank(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["ZADD", "test_rs:zrk", "10", "a", "20", "b", "30", "c"]).await?;
    let r = client.cmd(&["ZREVRANK", "test_rs:zrk", "c"]).await?;
    crate::assert_resp!(r, int(0), "ZREVRANK highest");
    let r = client.cmd(&["ZREVRANK", "test_rs:zrk", "a"]).await?;
    crate::assert_resp!(r, int(2), "ZREVRANK lowest");
    Ok(())
}

pub async fn test_zremrangebyrank(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["ZADD", "test_rs:zrr", "10", "a", "20", "b", "30", "c", "40", "d"]).await?;
    let r = client.cmd(&["ZREMRANGEBYRANK", "test_rs:zrr", "1", "2"]).await?;
    crate::assert_resp!(r, int(2), "ZREMRANGEBYRANK");
    let r = client.cmd(&["ZCARD", "test_rs:zrr"]).await?;
    crate::assert_resp!(r, int(2), "ZCARD after ZREMRANGEBYRANK");
    Ok(())
}

pub async fn test_zremrangebyscore(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["ZADD", "test_rs:zrs", "10", "a", "20", "b", "30", "c"]).await?;
    let r = client.cmd(&["ZREMRANGEBYSCORE", "test_rs:zrs", "15", "25"]).await?;
    crate::assert_resp!(r, int(1), "ZREMRANGEBYSCORE");
    Ok(())
}

pub async fn test_zrevrangebyscore(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["ZADD", "test_rs:zrb2", "1", "a", "2", "b", "3", "c", "4", "d"]).await?;
    let r = client.cmd(&["ZREVRANGEBYSCORE", "test_rs:zrb2", "4", "2"]).await?;
    crate::assert_resp!(r, arr_of_bulks(&["d", "c", "b"]), "ZREVRANGEBYSCORE");
    Ok(())
}
