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

// ── ZSet Set Operations (intersection / union / difference) ──────────────

pub async fn test_zinterstore_basic(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["FLUSHDB"]).await?;
    let _ = client.cmd(&["ZADD", "test_rs:zis1", "1", "a", "2", "b", "3", "c"]).await?;
    let _ = client.cmd(&["ZADD", "test_rs:zis2", "2", "b", "3", "c", "4", "d"]).await?;
    let r = client.cmd(&["ZINTERSTORE", "test_rs:zis_dest", "2", "test_rs:zis1", "test_rs:zis2"]).await?;
    crate::assert_resp!(r, int(2), "ZINTERSTORE count");
    let r = client.cmd(&["ZRANGE", "test_rs:zis_dest", "0", "-1"]).await?;
    crate::assert_resp!(r, arr_of_bulks(&["b", "c"]), "ZINTERSTORE members");
    Ok(())
}

pub async fn test_zinterstore_aggregate(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["FLUSHDB"]).await?;
    let _ = client.cmd(&["ZADD", "test_rs:zisa1", "1", "a", "2", "b"]).await?;
    let _ = client.cmd(&["ZADD", "test_rs:zisa2", "10", "a", "20", "b"]).await?;
    // SUM (default)
    let r = client.cmd(&["ZINTERSTORE", "test_rs:zisa_sum", "2", "test_rs:zisa1", "test_rs:zisa2"]).await?;
    crate::assert_resp!(r, int(2), "ZINTERSTORE SUM count");
    let r = client.cmd(&["ZSCORE", "test_rs:zisa_sum", "a"]).await?;
    match &r {
        RespType::BulkString(Some(data)) => {
            let s: i64 = String::from_utf8_lossy(data).parse().unwrap_or(0);
            if s != 11 { return Err(format!("expected 11, got {}", s)); }
        }
        _ => return Err("ZSCORE: expected BulkString".to_string()),
    }
    Ok(())
}

pub async fn test_zunionstore_basic(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["FLUSHDB"]).await?;
    let _ = client.cmd(&["ZADD", "test_rs:zus1", "1", "a", "2", "b"]).await?;
    let _ = client.cmd(&["ZADD", "test_rs:zus2", "3", "c", "4", "d"]).await?;
    let r = client.cmd(&["ZUNIONSTORE", "test_rs:zus_dest", "2", "test_rs:zus1", "test_rs:zus2"]).await?;
    crate::assert_resp!(r, int(4), "ZUNIONSTORE count");
    Ok(())
}

pub async fn test_zunionstore_weights(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["FLUSHDB"]).await?;
    let _ = client.cmd(&["ZADD", "test_rs:zusw1", "1", "a", "2", "b"]).await?;
    let _ = client.cmd(&["ZADD", "test_rs:zusw2", "3", "b", "4", "c"]).await?;
    let r = client.cmd(&["ZUNIONSTORE", "test_rs:zusw_dest", "2", "test_rs:zusw1", "test_rs:zusw2",
        "WEIGHTS", "2", "3"]).await?;
    crate::assert_resp!(r, int(3), "ZUNIONSTORE WEIGHTS count");
    // b from set1: 2*2=4, from set2: 3*3=9, sum=13
    let r = client.cmd(&["ZSCORE", "test_rs:zusw_dest", "b"]).await?;
    match &r {
        RespType::BulkString(Some(data)) => {
            let s: i64 = String::from_utf8_lossy(data).parse().unwrap_or(0);
            if s != 13 { return Err(format!("expected 13, got {}", s)); }
        }
        _ => return Err("ZSCORE: expected BulkString".to_string()),
    }
    Ok(())
}

pub async fn test_zinter_basic(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["FLUSHDB"]).await?;
    let _ = client.cmd(&["ZADD", "test_rs:zi1", "1", "a", "2", "b", "3", "c"]).await?;
    let _ = client.cmd(&["ZADD", "test_rs:zi2", "2", "b", "3", "c", "4", "d"]).await?;
    let r = client.cmd(&["ZINTER", "2", "test_rs:zi1", "test_rs:zi2"]).await?;
    crate::assert_resp!(r, arr_of_bulks(&["b", "c"]), "ZINTER basic");
    Ok(())
}

pub async fn test_zinter_withscores(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["FLUSHDB"]).await?;
    let _ = client.cmd(&["ZADD", "test_rs:ziws1", "1", "a", "2", "b"]).await?;
    let _ = client.cmd(&["ZADD", "test_rs:ziws2", "10", "a", "20", "b"]).await?;
    let r = client.cmd(&["ZINTER", "2", "test_rs:ziws1", "test_rs:ziws2", "WITHSCORES"]).await?;
    match &r {
        RespType::Array(Some(items)) => {
            if items.len() == 4 { Ok(()) }
            else { Err(format!("ZINTER WITHSCORES: expected 4 items, got {}", items.len())) }
        }
        _ => Err(format!("ZINTER WITHSCORES: expected Array, got {}", r)),
    }
}

pub async fn test_zunion_basic(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["FLUSHDB"]).await?;
    let _ = client.cmd(&["ZADD", "test_rs:zu1", "1", "a", "2", "b"]).await?;
    let _ = client.cmd(&["ZADD", "test_rs:zu2", "3", "c"]).await?;
    let r = client.cmd(&["ZUNION", "2", "test_rs:zu1", "test_rs:zu2"]).await?;
    crate::assert_resp!(r, arr_of_bulks(&["a", "b", "c"]), "ZUNION basic");
    Ok(())
}

pub async fn test_zunion_withscores(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["FLUSHDB"]).await?;
    let _ = client.cmd(&["ZADD", "test_rs:zuw1", "1", "a", "2", "b"]).await?;
    let _ = client.cmd(&["ZADD", "test_rs:zuw2", "3", "a"]).await?;
    let r = client.cmd(&["ZUNION", "2", "test_rs:zuw1", "test_rs:zuw2", "WITHSCORES"]).await?;
    match &r {
        RespType::Array(Some(items)) => {
            if items.len() == 4 { Ok(()) }
            else { Err(format!("ZUNION WITHSCORES: expected 4 items, got {}", items.len())) }
        }
        _ => Err(format!("ZUNION WITHSCORES: expected Array, got {}", r)),
    }
}

pub async fn test_zdiff_basic(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["FLUSHDB"]).await?;
    let _ = client.cmd(&["ZADD", "test_rs:zd1", "1", "a", "2", "b", "3", "c"]).await?;
    let _ = client.cmd(&["ZADD", "test_rs:zd2", "2", "b", "4", "d"]).await?;
    let r = client.cmd(&["ZDIFF", "2", "test_rs:zd1", "test_rs:zd2"]).await?;
    crate::assert_resp!(r, arr_of_bulks(&["a", "c"]), "ZDIFF basic");
    Ok(())
}

pub async fn test_zdiffstore_basic(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["FLUSHDB"]).await?;
    let _ = client.cmd(&["ZADD", "test_rs:zds1", "1", "a", "2", "b", "3", "c"]).await?;
    let _ = client.cmd(&["ZADD", "test_rs:zds2", "2", "b"]).await?;
    let r = client.cmd(&["ZDIFFSTORE", "test_rs:zds_dest", "2", "test_rs:zds1", "test_rs:zds2"]).await?;
    crate::assert_resp!(r, int(2), "ZDIFFSTORE count");
    let r = client.cmd(&["ZRANGE", "test_rs:zds_dest", "0", "-1"]).await?;
    crate::assert_resp!(r, arr_of_bulks(&["a", "c"]), "ZDIFFSTORE members");
    Ok(())
}

pub async fn test_zdiff_withscores(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["FLUSHDB"]).await?;
    let _ = client.cmd(&["ZADD", "test_rs:zdws1", "10", "a", "20", "b", "30", "c"]).await?;
    let _ = client.cmd(&["ZADD", "test_rs:zdws2", "20", "b"]).await?;
    let r = client.cmd(&["ZDIFF", "2", "test_rs:zdws1", "test_rs:zdws2", "WITHSCORES"]).await?;
    match &r {
        RespType::Array(Some(items)) => {
            if items.len() == 4 { Ok(()) }
            else { Err(format!("ZDIFF WITHSCORES: expected 4 items, got {}", items.len())) }
        }
        _ => Err(format!("ZDIFF WITHSCORES: expected Array, got {}", r)),
    }
}
