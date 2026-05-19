use crate::helpers::*;
use crate::RedisClient;

pub async fn test_sadd_new_key(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["SADD", "test_rs:s", "a", "b", "c"]).await?;
    crate::assert_resp!(r, int(3), "SADD new key");
    Ok(())
}

pub async fn test_sadd_existing_members(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SADD", "test_rs:ss2", "a", "b"]).await?;
    let r = client.cmd(&["SADD", "test_rs:ss2", "b", "c"]).await?;
    // b already exists, only c is new
    crate::assert_resp!(r, int(1), "SADD existing members");
    Ok(())
}

pub async fn test_sadd_duplicate(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SADD", "test_rs:s3", "x"]).await?;
    let r = client.cmd(&["SADD", "test_rs:s3", "x"]).await?;
    crate::assert_resp!(r, int(0), "SADD duplicate returns 0");
    Ok(())
}

pub async fn test_smembers(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SADD", "test_rs:s4", "a", "b", "c"]).await?;
    let r = client.cmd(&["SMEMBERS", "test_rs:s4"]).await?;
    match &r {
        mini_redis::resp::RespType::Array(Some(items)) => {
            if items.len() == 3 {
                Ok(())
            } else {
                Err(format!("SMEMBERS: expected 3 members, got {}", items.len()))
            }
        }
        _ => Err(format!("SMEMBERS: expected Array, got {}", r)),
    }
}

pub async fn test_smembers_empty_key(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["SMEMBERS", "test_rs:nosuchset"]).await?;
    crate::assert_resp!(r, empty_array(), "SMEMBERS empty key");
    Ok(())
}

pub async fn test_sismember_true(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SADD", "test_rs:s5", "member1"]).await?;
    let r = client.cmd(&["SISMEMBER", "test_rs:s5", "member1"]).await?;
    crate::assert_resp!(r, int(1), "SISMEMBER true");
    Ok(())
}

pub async fn test_sismember_false(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SADD", "test_rs:s6", "a"]).await?;
    let r = client.cmd(&["SISMEMBER", "test_rs:s6", "b"]).await?;
    crate::assert_resp!(r, int(0), "SISMEMBER false");
    Ok(())
}

pub async fn test_sismember_nonexistent_key(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["SISMEMBER", "test_rs:nokey", "x"]).await?;
    crate::assert_resp!(r, int(0), "SISMEMBER nonexistent key");
    Ok(())
}

pub async fn test_srem_single(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SADD", "test_rs:s7", "a", "b", "c"]).await?;
    let r = client.cmd(&["SREM", "test_rs:s7", "a"]).await?;
    crate::assert_resp!(r, int(1), "SREM single member");
    Ok(())
}

pub async fn test_srem_multiple(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SADD", "test_rs:s8", "x", "y", "z"]).await?;
    let r = client.cmd(&["SREM", "test_rs:s8", "x", "y"]).await?;
    crate::assert_resp!(r, int(2), "SREM multiple members");
    Ok(())
}

pub async fn test_srem_nonexistent(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["SREM", "test_rs:nokey", "x"]).await?;
    crate::assert_resp!(r, int(0), "SREM nonexistent key");
    Ok(())
}

pub async fn test_scard(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SADD", "test_rs:s9", "a", "b", "c"]).await?;
    let r = client.cmd(&["SCARD", "test_rs:s9"]).await?;
    crate::assert_resp!(r, int(3), "SCARD");
    Ok(())
}

pub async fn test_scard_empty(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["SCARD", "test_rs:nokey"]).await?;
    crate::assert_resp!(r, int(0), "SCARD empty key");
    Ok(())
}

pub async fn test_spop_single(client: &mut RedisClient) -> Result<(), String> {
    client.cmd(&["SADD", "test_rs:spo", "a"]).await?;
    let r = client.cmd(&["SPOP", "test_rs:spo"]).await?;
    crate::assert_resp!(r, bulk_str("a"), "SPOP single");
    let r = client.cmd(&["SCARD", "test_rs:spo"]).await?;
    crate::assert_resp!(r, int(0), "SCARD after SPOP");
    Ok(())
}

pub async fn test_spop_count(client: &mut RedisClient) -> Result<(), String> {
    client.cmd(&["SADD", "test_rs:spo2", "a", "b", "c"]).await?;
    let r = client.cmd(&["SPOP", "test_rs:spo2", "2"]).await?;
    assert!(matches!(&r, mini_redis::resp::RespType::Array(Some(v)) if v.len() == 2), "SPOP count 2");
    Ok(())
}

pub async fn test_spop_empty(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["SPOP", "test_rs:nokey"]).await?;
    crate::assert_resp!(r, null_bulk(), "SPOP empty");
    Ok(())
}

pub async fn test_srandmember_basic(client: &mut RedisClient) -> Result<(), String> {
    client.cmd(&["SADD", "test_rs:sran", "a"]).await?;
    let r = client.cmd(&["SRANDMEMBER", "test_rs:sran"]).await?;
    crate::assert_resp!(r, bulk_str("a"), "SRANDMEMBER");
    Ok(())
}

pub async fn test_sunion(client: &mut RedisClient) -> Result<(), String> {
    client.cmd(&["SADD", "test_rs:su1", "a", "b"]).await?;
    client.cmd(&["SADD", "test_rs:su2", "b", "c"]).await?;
    let r = client.cmd(&["SUNION", "test_rs:su1", "test_rs:su2"]).await?;
    assert!(matches!(&r, mini_redis::resp::RespType::Array(Some(v)) if v.len() == 3), "SUNION");
    Ok(())
}

pub async fn test_sinter(client: &mut RedisClient) -> Result<(), String> {
    client.cmd(&["SADD", "test_rs:si1", "a", "b", "c"]).await?;
    client.cmd(&["SADD", "test_rs:si2", "b", "c", "d"]).await?;
    let r = client.cmd(&["SINTER", "test_rs:si1", "test_rs:si2"]).await?;
    assert!(matches!(&r, mini_redis::resp::RespType::Array(Some(v)) if v.len() == 2), "SINTER");
    Ok(())
}

pub async fn test_sdiff(client: &mut RedisClient) -> Result<(), String> {
    client.cmd(&["SADD", "test_rs:sd1", "a", "b", "c"]).await?;
    client.cmd(&["SADD", "test_rs:sd2", "b"]).await?;
    let r = client.cmd(&["SDIFF", "test_rs:sd1", "test_rs:sd2"]).await?;
    assert!(matches!(&r, mini_redis::resp::RespType::Array(Some(v)) if v.len() == 2), "SDIFF");
    Ok(())
}

pub async fn test_smove(client: &mut RedisClient) -> Result<(), String> {
    client.cmd(&["SADD", "test_rs:sm1", "a", "b"]).await?;
    client.cmd(&["SADD", "test_rs:sm2", "c"]).await?;
    let r = client.cmd(&["SMOVE", "test_rs:sm1", "test_rs:sm2", "a"]).await?;
    crate::assert_resp!(r, int(1), "SMOVE success");
    let r = client.cmd(&["SISMEMBER", "test_rs:sm1", "a"]).await?;
    crate::assert_resp!(r, int(0), "SMOVE removed from source");
    let r = client.cmd(&["SISMEMBER", "test_rs:sm2", "a"]).await?;
    crate::assert_resp!(r, int(1), "SMOVE added to dest");
    Ok(())
}
